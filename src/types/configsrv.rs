
#[derive(Debug, Clone)]
pub struct ConfigSrv {
    pub bind: String,
    
    pub win_md5: md5::Digest,
    pub lin_md5: md5::Digest,
    pub non_md5: md5::Digest,
}

impl ConfigSrv {
    pub fn get_exec_filebytes(os :u8) -> Result<Vec<u8>,std::io::Error> {

        let mut win = std::env::args().nth(0).unwrap_or_default();
        if win.to_lowercase().ends_with(".exe") == false {
            win.push_str(".exe");
        }
        let lin = win.trim_end_matches(".exe").to_string();


        let filename = match os {
            1 => win,
            2 => lin,
            _ => "".to_string(),
        };

        let out = std::fs::read(filename)?;

        Ok(out)
    }


    pub fn setup(bind: String) -> ConfigSrv {
        let (win_md5,lin_md5,non_md5) =crate::core::md5_win_lin().unwrap();
        
        ConfigSrv {
            bind,
            win_md5,
            lin_md5,
            non_md5,
        }
    }
}
