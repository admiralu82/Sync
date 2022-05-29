use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::utils::listdirs::{rename_all_dir, testdir, testfile};

// use std::os::windows::prelude::MetadataExt;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub dir: bool,
    pub size: u64,
    pub date_mod: u64,
}

impl FileInfo {
    pub fn new_stop() -> Self {
        FileInfo {
            dir: true,
            name: "stop".to_string(),
            size: 111,
            date_mod: 0,
        }
    }
    pub fn is_stop(&self) -> bool {
        if self.dir && self.size == 111 && self.name == "stop".to_string() {
            return true;
        }
        false
    }

    fn from(fi: &std::fs::DirEntry) -> Option<FileInfo> {
        let name = fi.file_name();
        let name = name.to_str().unwrap().to_string();
        // .to_string_lossy().to_string();

        let m = fi.metadata();
        if let Err(err) = m {
            log::error!("---File Metadata error, skip file {} ---: {}", name, err);
            return None;
        }
        let m = m.unwrap();
        let dir = m.is_dir();
        let size = m.len();

        let mm = m.modified().unwrap_or_else(|_e| {
            let a = std::time::UNIX_EPOCH;
            a
        });
        let date_mod = mm.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        Some(FileInfo {
            name,
            dir,
            size,
            date_mod,
        })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(s: String) -> Self {
        serde_json::from_str(&s).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DirToArhiv {
    pub base: std::path::PathBuf, // c:\EFI
    pub alias: String,            // EFI
    pub path: std::path::PathBuf, // BOOT/BCD
    pub files: Vec<FileInfo>,

    pub login: String,
    pub id: String,
    pub hostname: String,
}

impl DirToArhiv {
    pub fn login_string(&self) -> String {
        format!("{}@{}@{}", self.login, self.hostname, self.id)
    }
    fn get_server_task_dir(&self, d: &str) -> PathBuf {
        let mut p = std::path::PathBuf::from(crate::types::DEFAULT_SERVER_STORE);

        p.push(format!("{}.{}", self.login_string(), d));
        p.push(&self.alias);

        p
    }
    pub fn base_version_dir(&self) -> PathBuf {
        self.get_server_task_dir("version")
    }
    pub fn base_sync_dir(&self) -> PathBuf {
        self.get_server_task_dir("sync")
    }
    pub fn full_dir(&self) -> PathBuf {
        let mut p = std::path::PathBuf::from(&self.base);
        p.push(&self.path);

        p
    }
    pub fn full_file(&self, f_s: &FileInfo) -> PathBuf {
        let mut p = self.full_dir();
        p.push(&f_s.name);

        p
    }
    pub fn vers_dir(&self) -> PathBuf {
        let mut p = self.base_version_dir();
        p.push(&self.path);

        p
    }
    pub fn vers_file(&self, f_s: &FileInfo) -> PathBuf {
        let mut p = self.vers_dir();

        let time = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let version_name = format!("{}.{}", f_s.name, time);
        p.push(&version_name);

        p
    }

    // на клиенте могут быть любые файлы, но сравниваем с обрезаным пробелом
    pub fn is_exist_on_client(&self, f_s: &FileInfo) -> bool {
        for f_c in &self.files {
            if f_c.name.trim_end() == f_s.name && f_c.dir == f_s.dir {
                return true;
            }
        }
        false
    }

    // f_s файл клиента поэтому обрезаем пробел
    pub fn is_file_eq(&self, f_c: &FileInfo) -> bool {
        // каталоги всегда бекапим
        if f_c.dir {
            return false;
        }
        for f_s in &self.files {
            if f_s.name == f_c.name.trim_end()
                && f_s.dir == f_c.dir
                && f_s.size == f_c.size
                && f_s.date_mod == f_c.date_mod
            {
                return true;
            }
        }
        false
    }

    // to_path - с убранными пробелами, исходный не меняем
    pub fn server_version_file(&self, f: &FileInfo, count_task: i32) {
        let mut from_path = self.full_file(f);
        let to_path = self.vers_file(f);

        // если имя содержит пробел на конце
        if f.name != f.name.trim_end() {
            let parent = from_path.parent();
            if parent.is_none() {
                log::error!("Error parent in {:?}",&from_path);
                return;
            };
            let parent = parent.unwrap().canonicalize();
            if let Err(err) = parent {
                log::error!("Error canonicalize in {:?}; {}",&from_path,err);
                return;
            }
            let mut parent = parent.unwrap();
            parent.push(&f.name);
            from_path = parent;
        }
       
        // проверяем каталог архивации файла
        let out = to_path.parent();
        if out.is_none() {
            log::error!("Error version (bad TO path): {:?}", &to_path);
            return;
        }

        let to_path_parent = out.unwrap();
        if testdir(&PathBuf::from(to_path_parent)) == false {
            let e = std::fs::create_dir_all(to_path_parent);
            log::info!("{}: vd-> : {:?} -> {:?}", count_task, to_path_parent, e);
        }

        // если архивируется файл
        if testfile(&from_path) {
            let _ = std::fs::remove_file(&to_path);
            let out = std::fs::rename(&from_path, &to_path);
            log::info!("{}: vf+> : {:?};   {:?}", count_task, &to_path, out);
            return;
        }

        if testdir(&from_path) {
            let out = rename_all_dir(&from_path, &to_path);
            log::info!("{}: vd+> : {:?};   {:?}", count_task, &to_path, out);
            return;
        }

        log::error!("Error version {:?} -> {:?}", from_path, to_path);
    }

    pub fn new(
        bb: PathBuf,
        pp: PathBuf,
        a: &String,
        login: &String,
        id: &String,
        hostname: &String,
    ) -> DirToArhiv {
        let mut out = DirToArhiv {
            base: bb,
            path: pp,
            alias: a.clone(),
            files: Vec::new(),

            login: login.clone(),
            id: id.clone(),
            hostname: hostname.clone(),
        };

        if a == &"".to_owned() {
            out.alias = out.base.file_name().unwrap().to_str().unwrap().to_string();
        } 

        out
    }
    pub fn list_files(&mut self) {
        let full_path = self.full_dir();
        self.files = Vec::new();

        let files_in_dir = std::fs::read_dir(&full_path);
        if let Err(err) = files_in_dir {
            log::error!("Error read_dir: {} => {:?}", err, &full_path);
            return;
        }
        let files_in_dir =  files_in_dir.unwrap();

        for f in files_in_dir {
            if let Err(err) = f {
                log::error! ("Error dir_entry : {} => {:?}", err, &full_path);
                continue;
            }

            let m = FileInfo::from(&f.unwrap());
            if m.is_none()  {
                continue;
            }
            self.files.push(m.unwrap());
        }
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    pub fn from_json(s: String) -> Self {
        serde_json::from_str(&s).unwrap()
    }
    pub fn new_stop(login: String, id: String, hostname: String) -> Self {
        let mut out = DirToArhiv {
            base: PathBuf::new(),
            path: PathBuf::new(),
            alias: "".to_string(),
            login,
            id,
            hostname,
            files: Vec::new(),
        };

        out.files.push(FileInfo::new_stop());
        out
    }
    pub fn is_stop(&self) -> bool {
        if self.files.len() == 1 {
            if self.files.get(0).unwrap().is_stop() {
                return true;
            }
        }

        false
    }
}

#[test]
fn test() {
    




    // std::fs::remove_dir_all(&b);

    // std::fs::rename(b, c);
}
