use structopt::StructOpt;

/// Command line options, which can be supplied during application start
#[derive(StructOpt)]
#[structopt(about = "Solana websocket server")]
pub struct CliOptions {
    /// Number of worker threads, to serve connection requests
    #[structopt(
        long = "worker-count",
        short,
        about = "number of worker threads, to serve connection requests"
    )]
    pub worker_count: Option<usize>,
    /// Number of threads responsible for managing subscriptions
    #[structopt(
        short,
        long = "manager-count",
        about = "number of threads responsible for managing subscriptions"
    )]
    pub manager_count: Option<usize>,
    /// List of addresses, where nsq lookup daemons can be queried, e.g. http://127.0.0.1:4161
    #[structopt(
        short,
        long,
        multiple = true,
        about = "list of addresses, where nsq lookup daemons can be queried, e.g. http://127.0.0.1:4161"
    )]
    pub nsqlookup: Vec<String>,
    /// Address, to which server should bind
    #[structopt(
        short = "l",
        long = "listen",
        about = "address, to which server should bind",
        default_value = "127.0.0.1:8080"
    )]
    pub bind_addr: String,
}
