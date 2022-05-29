use crate::types::{
    ConfigClnt, DirToArhiv, FileInfo, DEFAULT_CLIENT_RECONNECT_ERROR, DEFAULT_CLIENT_RETRYES,
};
// use rand::Rng;
use std::{path::PathBuf, time::Duration};
use tokio::{net::TcpStream, time::sleep};

use super::net::*;

pub async fn conn_loop(
    mut rx: tokio::sync::mpsc::Receiver<DirToArhiv>,
    mut tx_cfg: tokio::sync::mpsc::Sender<ConfigClnt>,
    mut cfg: ConfigClnt,
) -> Result<(), std::io::Error> {
    let mut error_count = DEFAULT_CLIENT_RETRYES;
    let mut last_dirtoarhiv = None;

    let mut count_task = 1;
    loop {
        // соединияемся, в случае ошибки повторяем
        let socket = TcpStream::connect(&cfg.server).await;
        if let Err(err) = socket {
            error_count -= 1;
            log::error!("Error ({}) in connect... {}", error_count, err);
            sleep(Duration::from_secs(DEFAULT_CLIENT_RECONNECT_ERROR)).await;
            if error_count == 0 {
                break;
            } else {
                continue;
            }
        }
        let mut socket = socket.unwrap();

        let out = loop_in(
            &mut socket,
            &mut cfg,
            &mut rx,
            &mut tx_cfg,
            last_dirtoarhiv.clone(),
            &mut count_task,
        )
        .await;
        if let Err(err) = out {
            error_count -= 1;
            last_dirtoarhiv = None;
            log::error!("Error ({}) in loop_in  {}", error_count, err);
            if error_count == 0 {
                break;
            } else {
                continue;
            }
        }
        last_dirtoarhiv = out.unwrap();

        match last_dirtoarhiv {
            Some(ref dir) => {
                error_count -= 1;
                log::error!(
                    "Error ({}) in do_net_tas {:?} -> {:?}",
                    error_count,
                    &dir.alias,
                    &dir.path
                );
                if error_count == 0 {
                    break;
                } else {
                    continue;
                }
            }
            None => break,
        }
    }
    rx.close();
    return Ok(());

    async fn loop_in(
        socket: &mut TcpStream,
        cfg: &mut ConfigClnt,
        rx: &mut tokio::sync::mpsc::Receiver<DirToArhiv>,
        tx_cfg: &mut tokio::sync::mpsc::Sender<ConfigClnt>,
        mut last_dirtoarhiv: Option<DirToArhiv>,
        count_task: &mut i32,
    ) -> Result<Option<DirToArhiv>, std::io::Error> {
        let out = hello_and_restart(socket)
            .await
            .and(update(socket).await)
            .and(auth(socket, cfg).await);

        if let Err(err) = out {
            log::error!("Error in starting: {}", err);
            return Err(err);
        }

        // если отправлияли уже конфигурацию то пропускаем
        if tx_cfg.is_closed() == false {
            let _ = tx_cfg.send(cfg.clone()).await;
        }

        // получем комманды от dir_explore
        loop {
            // пробуем выполнить последнее задание
            let d = if last_dirtoarhiv.is_some() {
                last_dirtoarhiv.take()
            } else {
                let d = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
                if d.is_err() {
                    log::error!("Error no command from dir_explore timeout");
                    break;
                }
                let d = d.unwrap();
                if d.is_none() {
                    break;
                }
                d
            }
            .unwrap();

            if d.is_stop() {
                break;
            }

            let out = do_net_task(socket, &d, count_task).await;
            *count_task += 1;
            if let Err(err) = out {
                log::error!("{}: TError {} -> {:?} : {}",*count_task-1, &d.alias, &d.path, err);
                return Ok(Some(d));
            }
        }
        finish_dirtoarhiv_packet(socket, cfg).await?;
        Result::<Option<DirToArhiv>, std::io::Error>::Ok(None)
    }

    async fn finish_dirtoarhiv_packet(
        socket: &mut TcpStream,
        cfg: &ConfigClnt,
    ) -> Result<(), std::io::Error> {
        log::info!("Send stop DirToArhiv ...");
        let out =
            DirToArhiv::new_stop(cfg.login.clone(), cfg.id.clone(), cfg.hostname.clone()).to_json();
        write_data(socket, out.as_bytes()).await?;

        log::info!("Send log ...");
        // посылаем лог
        let out = std::fs::read(crate::types::FILE_LOG)?;
        write_data(socket, &out).await?;
        Ok(())
    }
}

#[async_recursion::async_recursion]
pub async fn dir_explore(
    base: PathBuf,
    path: PathBuf,
    alias: String,

    cfg: &ConfigClnt,
    tx: &tokio::sync::mpsc::Sender<DirToArhiv>,
) {
    if tx.is_closed() {
        log::error!("Finish dir_explore. Tx is closed.");
        return;
    }

    let mut d = base.clone();
    d.push(&path);

    let files_in_dir = tokio::fs::read_dir(&d).await;
    if let Err(err) = files_in_dir {
        log::error!("Error list dir= {:?}: {}", &d, err);
        return;
    }
    let mut files_in_dir = files_in_dir.unwrap();

    let mut out = DirToArhiv::new(
        base.clone(),
        path.clone(),
        &alias,
        &cfg.login,
        &cfg.id,
        &cfg.hostname,
    );

    out.list_files();

    // отправляем себя
    let _ = tx.send(out).await;

    loop {
        let n = files_in_dir.next_entry().await;
        if let Err(err) = n {
            log::error!("Error list dir next_entry: {}", err);
            break;
        }
        let n = n.unwrap();
        if n.is_none() {
            break;
        }

        let n = n.unwrap();

        let info = n.metadata().await;
        if let Err(err) = info {
            log::error!("Error list dir metadata: {}", err);
            continue;
        }

        let info = info.unwrap();
        if info.is_dir() {
            let mut new_path = path.clone();
            new_path.push(n.file_name());
            dir_explore(base.clone(), new_path, alias.clone(), cfg, tx).await;
        }
    }
}

pub async fn run_client(cfg: ConfigClnt) {
    // let mut rng = rand::thread_rng();
    // let r = rng.gen_range(1..3);
    // log::info!("Client startup waiting {} min.", r);
    // sleep(Duration::from_secs(60 * r)).await;

    loop {
        do_client(cfg.clone()).await;
        // wait to reconnect
        log::info!(
            "Client fininsh. Waiting {} min...",
            crate::types::DEFAULT_CLIENT_RECONNECT / 60
        );
        sleep(Duration::from_secs(crate::types::DEFAULT_CLIENT_RECONNECT)).await;
    }
}

pub async fn do_client(cfg: ConfigClnt) {
    let (tx_cfg, mut rx_cfg) = tokio::sync::mpsc::channel::<ConfigClnt>(1);
    let (tx, rx) = tokio::sync::mpsc::channel::<DirToArhiv>(1);

    let l = conn_loop(rx, tx_cfg, cfg.clone());
    let conn = tokio::spawn(l);

    // получаем конфигурацию
    let cfg = tokio::time::timeout(Duration::from_secs(60), rx_cfg.recv()).await;
    if cfg.is_err() {
        log::error!("Error get cfg");
        return;
    }
    let cfg = cfg.unwrap();
    rx_cfg.close();

    if cfg.is_none() {
        log::error!("Error cfg is none");
        return;
    }
    let cfg = cfg.unwrap();
    log::info!("Client get config:\r\n{}", cfg.config);

    // обходим конфигурацию
    for (dir, alias) in cfg.config_lines() {
        if dir.trim().is_empty() {
            continue;
        }

        let base = PathBuf::from(dir);
        let path = PathBuf::from("");

        dir_explore(base, path, alias, &cfg, &tx).await;
    }

    // send stop DirToArhiv
    let out = DirToArhiv::new_stop(cfg.login.clone(), cfg.id.clone(), cfg.hostname.clone());
    let _ = tx.send(out).await;

    let out = tokio::join!(conn);
    log::info!("{:?}", out);
}

// #[async_recursion::async_recursion]
async fn do_net_task(
    socket: &mut tokio::net::TcpStream,
    files_to_upload: &DirToArhiv,
    count_task: &mut i32,
) -> Result<(), std::io::Error> {
    // берем c сервера что обновлять
    let files_befour = files_to_upload.files.len();

    // send list files to server
    let json = files_to_upload.to_json();
    write_data(socket, json.as_bytes()).await?;

    // recv list files to update from server
    let a = read_data_in_string(socket).await?;
    let files_to_upload = DirToArhiv::from_json(a);

    // do some statistic
    let files_after = files_to_upload.files.len();
    let mut files_after_dirs = 0;
    for f in &files_to_upload.files {
        if f.dir {
            files_after_dirs += 1;
        }
    }

    // information
    log::info!(
        "{}: {} -> {:?}\\{:?} :: {}f + {}d = {}",
        count_task,
        files_to_upload.alias,
        files_to_upload.base,
        files_to_upload.path,
        files_after - files_after_dirs,
        files_after_dirs,
        files_befour,
    );

    // обновляем файлы
    for f in &files_to_upload.files {
        send_fileinfo_to_server(socket, &files_to_upload, f, count_task).await?;
    }
    // финишный пакет
    send_fileinfo_to_server(socket, &files_to_upload, &FileInfo::new_stop(), count_task).await?;
    return Ok(());

    async fn send_fileinfo_to_server(
        socket: &mut TcpStream,
        files: &DirToArhiv,
        f: &FileInfo,
        count_task: &mut i32,
    ) -> Result<(), std::io::Error> {
        if f.is_stop() {
            write_data(socket, f.to_json().as_bytes()).await?;
            return Ok(());
        }

        if f.dir {
            return Ok(());
        }

        // open file
        let p = files.full_file(f);
        let file = tokio::fs::File::open(&p).await;

        if let Err(err) = file {
            log::error!("{}: File {:?} open error: {}. Skeep file.",count_task, p, err);
            return Ok(());
        }
        let file = file.unwrap();

        // send FileInfo to server
        write_data(socket, f.to_json().as_bytes()).await?;
        log::info!("{}: --> {} (len={})",count_task, f.name, f.size);
        stream_send_file(socket, file).await?;
        log::info!("{}: +-> {} (len={})",count_task, f.name, f.size);
        Ok(())
    }
}

async fn hello_and_restart(socket: &mut TcpStream) -> Result<(), std::io::Error> {
    // hello message /////////////////////////////////////////////////////
    write_string_max32(socket, crate::types::MAGIC_NUMBER.to_string()).await?;
    read_status(socket).await?;
    // no restart message /////////////////////////////////////////////////////
    write_status_ok(socket).await?;
    log::info!("Hello...");
    Ok(())
}
async fn update(socket: &mut TcpStream) -> Result<(), std::io::Error> {
    let md5 = crate::core::md5_self();
    let win_lin;
    if cfg!(windows) {
        win_lin = 1u8;
    } else {
        win_lin = 2u8;
    }

    write_status(socket, win_lin).await?;
    write_data(socket, &md5.0[..]).await?;

    let out = read_status(socket).await?;
    if out == crate::types::DEFAULT_UPDATE {
        // download update
        log::info!("Self update  begin...");
        let buf = read_data(socket).await?;

        log::info!("Self update save update... size={}", buf.len());

        // формируем пути
        let file_name = ConfigClnt::get_filename();
        let mut file_name_new = file_name.clone();
        file_name_new.push_str(".new");
        let mut file_name_old = file_name.clone();
        file_name_old.push_str(".old");

        // сохраняем обновление
        std::fs::write(&file_name_new, &buf)?;
        sleep(std::time::Duration::from_secs(1)).await;

        std::fs::rename(&file_name, &file_name_old)?;
        std::fs::rename(&file_name_new, &file_name)?;

        // запускаем сами себя
        let mut cmd = std::process::Command::new(&file_name);
        let mut a: Vec<String> = std::env::args().collect();
        a.remove(0);

        log::info!("Restarting: {} {:?}", &file_name, a);

        let a = cmd.args(a).spawn();
        if let Err(err) = a {
            log::error!("Error to self spawn : {}", err);
        }

        std::process::exit(0);
    }

    log::info!("Update...");
    Ok(())
}
async fn auth(socket: &mut TcpStream, cfg: &mut ConfigClnt) -> Result<(), std::io::Error> {
    if cfg.id == "" {
        write_data(socket, "?".as_bytes()).await?;
        cfg.id = read_data_in_string(socket).await?;

        tokio::fs::write(crate::types::FILE_CLIENT_ID, cfg.id.as_bytes()).await?;

        if cfg.id == "" {
            log::error!("Client get ID  error {}", cfg.id);
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        } else {
            log::info!("Client get new ID: {}", cfg.id);
        }
    }

    // write login stirng
    super::net::write_data(socket, cfg.login_string().as_bytes()).await?;

    let a = crate::utils::listdirs::list_dirs(3, &crate::utils::listdirs::EXCLUE_DIRS_DEFAULT());
    let a = crate::utils::listdirs::directorys_info_to_string_part2(&a);

    // write list dirs to server
    write_data(socket, a.as_bytes()).await?;

    // read config from server
    let config = super::net::read_data_in_string(socket).await?;
    cfg.set_config(config);
    log::info!("Auth...");
    Ok(())
}
