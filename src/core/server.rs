use crate::{
    types::{DirToArhiv, FileInfo},
    utils::listdirs::{testdir, testfile, trim_endspaces_in_files},
};

use super::net::*;

lazy_static::lazy_static! {
    static ref ID: crate::core::IdGenerator = crate::core::IdGenerator::new();
}

pub async fn run_server(
    cfg: crate::types::ConfigSrv,
    socket: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
) {
    let out = worker_server(cfg, socket).await;

    if let Err(err) = out {
        log::info!(
            "Stop server worker  from {}. Error: {}",
            addr.to_string(),
            err
        );
    }
}

//

pub async fn worker_server(
    cfg: crate::types::ConfigSrv,
    mut socket: tokio::net::TcpStream,
) -> Result<(), std::io::Error> {
    log::info!("Starting server worker from {}", socket.peer_addr()?);

    hello_and_restart(&cfg, &mut socket).await?;
    update(&cfg, &mut socket).await?;
    auth(&cfg, &mut socket).await?;

    // backup function /////////////////////////
    let mut count_task = 1;

    loop {
        let sync_files_on_client = read_data_in_string(&mut socket).await?;
        let mut sync_files_on_client = DirToArhiv::from_json(sync_files_on_client);

        if sync_files_on_client.is_stop() {
            // читем лог после финального пакета
            log::info!("Read log for {} ...", sync_files_on_client.login_string());
            let log = read_data(&mut socket).await?;

            // сохраняем лог
            let mut p = sync_files_on_client.base_sync_dir();
            let _ = std::fs::create_dir_all(&p);
            p.push(crate::types::FILE_LOG);
            std::fs::write(p, log)?;
            log::info!(
                "Stop server worker from {}",
                sync_files_on_client.login_string()
            );
            break;
        }

        log::info!(
            "{}: {} -> {:?} :: ({})  ({})",
            count_task,
            sync_files_on_client.alias,
            sync_files_on_client.path,
            sync_files_on_client.files.len(),
            sync_files_on_client.login_string(),
        );

        // делаем локальные задачи
        let sync_files_on_server = server_local_task(&sync_files_on_client, count_task).await?;

        // readkey();
        // information
        let mut dirs_count = 0;
        for f in &sync_files_on_client.files {
            if f.dir {
                dirs_count += 1;
                continue;
            }
        }
        log::info!(
            "{}: {} -> {:?} :: {}f + {}d < ({}) ({})",
            count_task,
            sync_files_on_server.alias,
            sync_files_on_server.path,
            sync_files_on_server.files.len() - dirs_count,
            dirs_count,
            sync_files_on_client.files.len(),
            sync_files_on_client.login_string(),
        );

        // send response
        sync_files_on_client.files = sync_files_on_server.files.clone();
        super::net::write_data(&mut socket, sync_files_on_client.to_json().as_bytes()).await?;

        // ждем посылки файлов. FileInfo
        loop {
            // ловим пакет FileInfo
            let mut packet = FileInfo::from_json(read_data_in_string(&mut socket).await?);

            // Если это финишный то отвечем ОК и выходим
            if packet.is_stop() {
                break;
            }

            // убираем пробелы в конце имени
            packet.name = packet.name.trim_end().to_string();

            // готовим путь
            let p = sync_files_on_server.full_file(&packet);
            // убираем пробелы в пути
            let p = trim_endspaces_in_files(p);

            // если файл существует то создаем его версию
            if testfile(&p) {
                sync_files_on_server.server_version_file(&packet, count_task);
            }

            // получаем файл
            log::info!("{}: <--r : {} ({})", count_task, packet.name, packet.size);
            server_recv_file(&mut socket, &p, packet.size).await?;
            log::info!("{}: <-+r : {} ({})", count_task, packet.name, packet.size);

            // ставим время файлу
            let time = filetime::FileTime::from_unix_time(packet.date_mod as i64, 0);
            let out = filetime::set_file_mtime(&p, time);
            if let Err(err) = out {
                log::error!("Error set time: {}", err);
            }
        }
        // как отправили все файлы в задании
        count_task += 1;
    }
    return Ok(());
}

async fn hello_and_restart(
    _cfg: &crate::types::ConfigSrv,
    socket: &mut tokio::net::TcpStream,
) -> Result<(), std::io::Error> {
    // <------- magic
    let msg = super::net::read_string_max32(socket).await?;

    if msg != crate::types::MAGIC_NUMBER.to_string() {
        log::error!("Bad auth from {}", socket.peer_addr()?);
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    // -------> Ok
    super::net::write_status_ok(socket).await?;

    // restart message // <------- ok, !ok - resatrt
    let st = super::net::read_status(socket).await?;
    if st != crate::types::DEFAULT_OK {
        // std::process::exit(0);
        todo!("restart server not implemented");
    }
    Ok(())
}

async fn update(
    cfg: &crate::types::ConfigSrv,
    socket: &mut tokio::net::TcpStream,
) -> Result<(), std::io::Error> {
    // update message /////////////////////////////////////////////////////
    let win_lin = read_status(socket).await?;
    let out = read_data(socket).await?;

    // с чем будем сравнивать
    let match_md5 = match win_lin {
        1 => cfg.win_md5,
        2 => cfg.lin_md5,
        _ => {
            log::error!("Bad update sys from {}", socket.peer_addr()?);
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
    };

    let rcv_md5 = md5::Digest(out.as_slice().try_into().unwrap());
    if rcv_md5 == match_md5 || rcv_md5 == cfg.non_md5 {
        super::net::write_status_ok(socket).await?;
    } else {
        log::info!(
            "Clent need update os={} rcv_md5={:?} match_md5={:?} from {}",
            win_lin,
            rcv_md5,
            match_md5,
            socket.peer_addr()?
        );
        super::net::write_status(socket, crate::types::DEFAULT_UPDATE).await?;
        // send size and update
        let out_update_bytes = crate::types::ConfigSrv::get_exec_filebytes(win_lin)?;
        super::net::write_data(socket, &out_update_bytes).await?;

        log::info!(
            "Update (size={}) OK for {}",
            out_update_bytes.len(),
            socket.peer_addr()?
        );
    }

    Ok(())
}

async fn auth(
    _cfg: &crate::types::ConfigSrv,
    socket: &mut tokio::net::TcpStream,
) -> Result<String, std::io::Error> {
    let mut auth: String;
    loop {
        // let n = socket.read(buf_rcv).await?;
        // auth = std::str::from_utf8(&buf_rcv[..n]).unwrap().to_string();
        auth = read_data_in_string(socket).await?;

        if auth == "?" {
            // send new ID
            let new_id = ID.generate().to_string();
            write_data(socket, new_id.as_bytes()).await?;
            log::info!("Server send new ID {} for {}", new_id, socket.peer_addr()?);
            continue;
        }
        break;
    }
    log::info!("Client auth: {}, from: {}", auth, socket.peer_addr()?);

    // read cfg dirs from client
    let ss = read_data(socket).await?;

    // save cfg
    let mut path_for_client_config = std::path::PathBuf::from(crate::types::DEFAULT_SERVER_STORE);
    path_for_client_config.push(&auth);

    if testfile(&path_for_client_config) == false {
        std::fs::write(&path_for_client_config, "".as_bytes())?;
    }

    let old_file = std::fs::read_to_string(&path_for_client_config).unwrap_or_default();
    let pos = old_file
        .find(crate::types::FILE_CONFIG_SEPARATOR)
        .unwrap_or_else(|| old_file.len());
    let (mut part1, _) = old_file.split_at(pos);

    if part1.len() == 0 {
        part1 = " ";
    }

    let part1 = part1.as_bytes();

    let mut file = std::fs::File::create(&path_for_client_config)?;
    std::io::Write::write_all(&mut file, part1)?;
    std::io::Write::write_all(&mut file, ss.as_slice())?;
    std::io::Write::flush(&mut file)?;
    log::info!("Client file: {} is updated", auth);

    // send config
    crate::core::net::write_data(socket, part1).await?;

    Ok(auth)
}

async fn server_local_task(
    files_on_client: &DirToArhiv,
    count_task: i32,
) -> Result<DirToArhiv, std::io::Error> {
    // создаем каталоги задания
    {
        let d1 = files_on_client.base_sync_dir();
        if testdir(&d1) {
            let _ = std::fs::create_dir_all(&d1);
        }
        let d1 = files_on_client.base_version_dir();
        if testdir(&d1) {
            let _ = std::fs::create_dir_all(&d1);
        }
    }

    // берем файлы сервера на основе данных клиента

    let mut files_on_server = files_on_client.clone();
    files_on_server.base = files_on_server.base_sync_dir();
    files_on_server.path = trim_endspaces_in_files(files_on_server.path);

    // создадим каталог задания
    let task_dir = files_on_server.full_dir();

    if testdir(&task_dir) == false {
        let e = std::fs::create_dir_all(&task_dir);
        log::info!("{}: d+-> : {:?}  ({:?})", count_task, task_dir, e);
    }
    files_on_server.list_files();

    //// если файлы и каталоги из server нет на клиенте то бакапим их.
    let mut files_to_stay = Vec::new();
    for f_s in &files_on_server.files {
        if files_on_client.is_exist_on_client(f_s) {
            files_to_stay.push(f_s.clone());
        } else {
            files_on_server.server_version_file(f_s, count_task);
        }
    }
    files_on_server.files = files_to_stay;

    // убираем из архивации совпадающие файлы
    let mut files_to_arh = Vec::new();
    for f_c in &files_on_client.files {
        if files_on_server.is_file_eq(f_c) {
            continue;
        }
        files_to_arh.push(f_c.clone());
    }
    files_on_server.files = files_to_arh;

    // dbg!(&files_on_server.files);
    // readkey();

    Ok(files_on_server)
}

#[test]
fn test() {}
