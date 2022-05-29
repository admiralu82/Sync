
#[derive(clap::Parser, Debug)]
pub struct Cli {
    #[clap(short, long, parse(from_occurrences))]
    pub verbose: usize, 
    #[clap(subcommand)]
    pub command: Option<Commands>,
}


#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// does testing things
    Server {
        /// bind to interface
        #[clap(default_value=crate::types::DEFAULT_SERVER)]
        bind: String, 
    },
    /// does client things
    Client {
        /// name of client
        login: String,

        #[clap(default_value=crate::types::DEFAULT_SERVER)]
        /// addres of server
        server: String,        
    }
}
