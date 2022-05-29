use std::path::PathBuf;

#[allow(non_snake_case)]
pub fn EXCLUE_DIRS_DEFAULT() -> Vec<std::ffi::OsString> {
    let a = std::ffi::OsString::from("WinSxS");
    let b = std::ffi::OsString::from("SysWOW64");
    let c = std::ffi::OsString::from("System32");
    let d = std::ffi::OsString::from("Temp");

    vec![a, b, c, d]
}

type DirectorysInfo = Vec<std::fs::DirEntry>;

pub fn directorys_info_to_string_part2(dd: &DirectorysInfo) -> String {
    let mut ss = String::with_capacity(2_000_000);
    ss.push_str(crate::types::FILE_CONFIG_SEPARATOR);
    ss.push_str("\r\n");

    dd.iter().for_each(|d| {
        let a = d.path();
        let a = a.as_os_str();
        let a = a.to_str();

        if a.is_some() {
            ss.push_str(a.unwrap());
            ss.push_str("\r\n");
        }
    });
    ss
}

pub fn list_dirs(deep: i8, dir_exclude: &Vec<std::ffi::OsString>) -> DirectorysInfo {
    let drives: &str;
    if cfg!(windows) {
        drives = "abcdefghijklmnopqrstuvwxyz";
    } else {
        drives = "/";
    }

    let mut out: Vec<std::fs::DirEntry> = vec![];

    for j in drives.as_bytes() {
        let mut path = (*j as char).to_string();

        if cfg!(windows) {
            path.push_str(":\\");
        } else {
        }

        let path = std::path::Path::new(&path);
        out.append(&mut list_dir(path, deep, dir_exclude));
    }

    out
}

pub fn list_dir(
    d: &std::path::Path,
    deep: i8,
    dir_exclude: &Vec<std::ffi::OsString>,
) -> DirectorysInfo {
    let mut out: DirectorysInfo = vec![];

    if deep == 0 {
        return out;
    }

    let f = std::fs::read_dir(d);
    if let Err(_) = f {
        // println!("Error reading directory: {}", e);
        return out;
    }

    for i in f.unwrap() {
        let d = crate::if_result_fail_continue!(i);

        if d.path().is_dir() {
            let d_tmp = d.path();
            let file_name = d_tmp.file_name();
            let file_name = crate::if_option_fail_continue!(file_name);

            if dir_exclude.contains(&std::ffi::OsString::from(file_name)) {
                continue;
            }

            out.push(d);
            let mut ddd = list_dir(d_tmp.as_path(), deep - 1, dir_exclude);
            out.append(&mut ddd);
        }
    }
    out
}

pub fn trim_endspaces_in_files(old: PathBuf) -> PathBuf {
    let mut new = PathBuf::new();

    for i in old.iter() {
        let s = i.to_str();
        if s.is_none() {
            log::error!("!!! Error in path {:?} in {:?}", &old, i);
            return old;
        }
        let s = s.unwrap();
        let s_new = s.trim_end();

        new.push(s_new);
    }

    new
}
pub fn readkey() {
    let mut line = String::new();
    let _input = std::io::stdin()
        .read_line(&mut line)
        .expect("Failed to read line");
}
pub fn testfile(p: &std::path::PathBuf) -> bool {
    let info = std::fs::metadata(p);
    if info.is_err() {
        return false;
    }

    let info = info.unwrap();
    info.is_file()
}
pub fn testdir(p: &std::path::PathBuf) -> bool {
    let info = std::fs::metadata(p);
    if info.is_err() {
        return false;
    }

    let info = info.unwrap();
    info.is_dir()
}

pub fn rename_all_dir(from: &PathBuf, to: &PathBuf) -> Result<(),std::io::Error> {
    // if testdir(&to) {
    //     let _ = std::fs::remove_dir_all(&to)?;
    // }
    
    move_dir(from.clone(), to.clone())?;
    
    if testdir(&from) {
        std::fs::remove_dir_all(from)?;
    }
    
    return Ok(());

    // let out = std::fs::rename(from, to);
    fn move_dir(from: PathBuf, to: PathBuf) -> Result<(), std::io::Error> {
        let files = std::fs::read_dir(&from)?;
        if testdir(&to)==false {
            std::fs::create_dir_all(&to)?;
        }
        
        for f in files {
            if let Err(err) = f {
                log::error!("Version err: {}", err);
                continue;
            }
            let f = f.unwrap();

            if f.metadata()?.is_file() {
                let mut to_file = to.clone();
                to_file.push(f.file_name());
                std::fs::rename(f.path(), to_file)?;
                continue;
            }

            let mut to_dir = to.clone();
            to_dir.push(f.file_name());
            move_dir(PathBuf::from(f.path()), to_dir)?;
        }

        Ok(())
    }
}

#[test]
fn ts() {
    
}
