#[derive(Debug, Clone)]
pub struct ConfigClnt {
    pub login: String,
    pub server: String,
    pub config: String,

    pub id: String,
    pub hostname: String,
}

impl ConfigClnt {
    pub fn refresh_id(&mut self) {
        self.id = std::fs::read_to_string(crate::types::FILE_CLIENT_ID)
            .unwrap_or_default()
            .trim()
            .to_string();
    }

    pub fn setup(login: String, server: String) -> ConfigClnt {
        let hostname = hostname::get().unwrap_or_default();
        let hostname = hostname.to_str().unwrap_or_default().to_string();

        let mut out = ConfigClnt {
            login,
            id: "".to_string(),
            server,
            config: "".to_owned(),
            hostname,
        };
        out.refresh_id();

        out
    }

    pub fn get_filename() -> String {
        let file_name = std::env::args().nth(0).unwrap();

        let file_name = std::path::Path::new(&file_name)
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let mut file_name = file_name.to_string();

        let out: String = file_name.clone();

        if cfg!(windows) {
            if file_name.to_lowercase().ends_with(".exe") == false {
                file_name.push_str(".exe")
            }
        }

        if std::fs::metadata(std::path::Path::new(&file_name)).is_err() {
            return out;
        } else {
            file_name
        }
    }

    pub fn read_from_filename() -> Option<ConfigClnt> {
        let args: Vec<String> = std::env::args().collect();

        let file_name = std::path::Path::new(&args[0])
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        let file_name = file_name.trim_end_matches(".exe");
        let s = file_name.split("_").nth(1).unwrap_or_default();
        let s = s.replace("!", ":");

        let cfg = ConfigClnt::parse_str(&s);
        match cfg {
            Some(cfg) => {
                log::info!("Ok config readFromFileName: {}", file_name);
                return Some(cfg);
            }
            None => {
                log::error!("Error config readFromFileName : {}", file_name);
                log::error!(
                    "ReadFromFileName pattern: syncFileExecute_login[@server[!port]][.exe]"
                );
                return None;
            }
        }
    }

    fn parse_str(s: &str) -> Option<ConfigClnt> {
        let s = s.trim();

        // parse string -  login@servername:port
        let mut it = s.split("@");
        let login = it.next().unwrap_or_default().to_string();
        let mut server = it.next().unwrap_or_default();

        if login == "" {
            return None;
        }

        if server == "" {
            server = crate::types::DEFAULT_SERVER;
        }

        let server = if server.contains(":") == false {
            let mut srv = server.to_string();
            let port = format!(":{}", crate::types::DEFAULT_SERVER_PORT);
            srv.push_str(&port);

            srv
        } else {
            server.to_string()
        };

        let out = ConfigClnt::setup(login, server);

        Some(out)
    }

    pub fn set_config(&mut self, s: String) {
        self.config = s;
    }

    // pub fn config_lines(&self) -> Vec<String> {
    //     let out = self.config.clone();
    //     let out: Vec<String> = out.lines().map(|s| s.to_owned()).collect();
    //     out
    // }
    pub fn config_lines(&self) -> Vec<(String, String)> {
        let out = self.config.clone();
        let out  = out
            .lines()
            .map(|s| {
                let pos = s.find("#");
                if pos.is_none() {
                    return (s.to_string(),"".to_string())
                }
                let pos = pos.unwrap();
                let out = s.split_at(pos);
                let out  = ( out.0.trim().to_string(), out.1[1..].trim().to_string());
                
                out
            })
            .collect();
        out
    }

    pub fn login_string(&self) -> String {
        format!("{}@{}@{}", self.login, self.hostname, self.id)
    }
}

#[test]
fn parse_str_test() {
    // ConfigClnt::new().parse_str("  asd  login@server");
}
