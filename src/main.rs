#![windows_subsystem = "windows"]

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let cli = <sync::utils::Cli as clap::Parser>::parse();
    setup_logger(cli.verbose).unwrap();

    match cli.command {
        Some(a) => {
            match a {
                sync::utils::Commands::Server { bind } => {
                    // берем настройки и запускаем сервер
                    let cfg = sync::types::ConfigSrv::setup(bind);
                    let listener = tokio::net::TcpListener::bind(&cfg.bind).await;

                    if let Err(err) = listener {
                        log::error!("Error start Server listener: {}", err);
                        std::process::exit(0);
                    }

                    log::info!("Server start: bind={} win={:?} lin={:?}", &cfg.bind,&cfg.win_md5, &cfg.lin_md5);

                    std::fs::create_dir_all(sync::types::DEFAULT_SERVER_STORE)?;

                    let listener = listener.unwrap();
                    loop {
                        let acc = listener.accept().await;
                        if let Err(err) = acc {
                            log::error!("Server accept conn error: {}", err);
                            continue;
                        }

                        let (socket, addr) = acc.unwrap();
                        
                        log::info!("Server accept conn: {}", &addr);
                        let server_handle = sync::core::server::run_server(cfg.clone(), socket, addr);
                        tokio::spawn(server_handle);
                    }
                }
                sync::utils::Commands::Client {
                    login,
                    server,
                } => {
                    // берем настройки из команднй строки  и запускаем клиента
                    let cfg = sync::types::ConfigClnt::setup(login, server);

                    log::info!("Client start: {:?}", &cfg);
                    sync::core::client::run_client(cfg).await;
                }
            }
        }
        None => {
            // запускаем клиента по умолчанию с настройками из имени файла
            let cfg_from_filename = sync::types::ConfigClnt::read_from_filename();

            match cfg_from_filename {
                Some(ccc) => {
                    log::info!("Client DEFAULT start: {:?}", &ccc);
                    sync::core::client::run_client(ccc).await;
                }
                None => {
                    log::error!("Client DEFAULT start error. Check filename.");
                    std::process::exit(1);
                }
            }
        }
    }
    log::info!("Exit program");

    Ok(())
}

fn setup_logger(level: usize) -> Result<(), fern::InitError> {

    let a = std::fs::metadata(sync::types::FILE_LOG);
    if let Ok(m) = a {
        if m.len() > 2_000_000 {
            let mut log_old = sync::types::FILE_LOG.to_string();
            log_old.push_str(".old");

            let _ = std::fs::rename(sync::types::FILE_LOG, log_old);
        }
    }

    let log_level;
    match level {
        0..=1 => log_level = log::LevelFilter::Info,
        2 => log_level = log::LevelFilter::Warn,
        3 => log_level = log::LevelFilter::Error,
        _ => log_level = log::LevelFilter::Off,
    }

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(sync::types::FILE_LOG)?)
        .apply()?;

    log::info!("Log init: {}", log_level);
    Ok(())
}

