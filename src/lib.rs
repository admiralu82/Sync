
pub mod types {

    pub const BUFFER_SIZE: usize = 4096;

    pub const FILE_CLIENT_ID: &str = "sync.id";
    pub const FILE_LOG: &str = "sync.log";
    pub const FILE_CONFIG_SEPARATOR: &str =
        "\r\n-------------@@@----------SEPARATOR DO NOT EDIT----------@@@-------------";

    pub const MAGIC_NUMBER: &str = "Z53Yj@e29";

    pub const DEFAULT_SERVER_STORE: &str = "_SYNC_";
    // pub const DEFAULT_SERVER: &str = "localhost:12346";
    pub const DEFAULT_SERVER: &str = "backup.lab31.ru:12345";
    pub const DEFAULT_SERVER_PORT: i32 = 12345;
    
    pub const DEFAULT_CLIENT_RECONNECT: u64 = 60*60*3;
    pub const DEFAULT_CLIENT_RECONNECT_ERROR: u64 = 60*1;
    pub const DEFAULT_CLIENT_RETRYES: u32 = 15;

    pub const DEFAULT_OK: u8 = 100;
    pub const DEFAULT_UPDATE: u8 = 101;
    pub const DEFAULT_ERROR: u8 = 200;

    mod configclnt;
    pub use self::configclnt::*;

    mod configsrv;
    pub use self::configsrv::*;

    mod dirtoarhiv;
    pub use self::dirtoarhiv::*;
}

pub mod core {
    pub mod client;
    pub mod server;

    mod auth;
    pub use self::auth::*;

    pub mod net;
}

pub mod utils {
    pub mod listdirs;
    // pub use self::listdirs::*;

    mod macros;
    // pub use self::macros::*;

    mod cli;
    pub use self::cli::*;
}
