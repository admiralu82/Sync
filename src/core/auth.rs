

const SERVER_ID_FILE_NAME: &str = ".id_last";
const SERVER_ID_START: u32 = 100;

pub struct IdGenerator(std::sync::Mutex<u32>);

impl IdGenerator {
    pub fn new() -> IdGenerator {
        let id: u32;

        match std::fs::read_to_string(SERVER_ID_FILE_NAME) {
            Ok(s) => {
                id = s.parse::<u32>().unwrap();
            }
            Err(_) => {
                let _ = std::fs::write(SERVER_ID_FILE_NAME, SERVER_ID_START.to_string());
                id = SERVER_ID_START;
            }
        }

        IdGenerator(std::sync::Mutex::new(id))
    }

    pub fn current(&self) -> u32 {
        *self.0.lock().unwrap()        
    }

    pub fn generate(&self) -> u32 {
        let mut l = self.0.lock().unwrap();

        *l += 1;
        let _ = std::fs::write(SERVER_ID_FILE_NAME, (*l).to_string());

        *l
    }
}

pub fn md5_self() -> md5::Digest {
    let mut a = std::env::args().nth(0).unwrap_or_default();

    if cfg!(windows) {
        if a.to_lowercase().ends_with(".exe") == false {
            a.push_str(".exe");
        }
    }

    md5_of_file(&a).unwrap_or(md5::Digest([0u8; 16]))
}

pub fn md5_win_lin() -> Result<(md5::Digest, md5::Digest, md5::Digest), std::io::Error> {

    let mut win = std::env::args().nth(0).unwrap_or_default();
    if win.to_lowercase().ends_with(".exe") == false {
        win.push_str(".exe");
    }
    let lin = win.trim_end_matches(".exe");

    let win_md5 = md5_of_file(&win);
    let lin_md5 = md5_of_file(&lin);

    let win_md5 = win_md5.unwrap_or(md5::Digest([0u8; 16]));
    let lin_md5 = lin_md5.unwrap_or(md5::Digest([0u8; 16]));

    Ok((win_md5, lin_md5, md5::Digest([0u8; 16])))
}

fn md5_of_file(s: &str) -> Result<md5::Digest, std::io::Error> {
    let b = std::fs::read(s)?;
    let md5 = md5::compute(b);
    Ok(md5)
}

