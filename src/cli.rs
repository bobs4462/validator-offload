use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Solana websocket server")]
pub struct CliOptions {
    #[structopt(
        long = "worker-count",
        short,
        about = "number of worker threads, to serve connection requests"
    )]
    pub worker_count: Option<usize>,
    #[structopt(
        short,
        long = "manager-count",
        about = "number of threads responsible for managing subscriptions"
    )]
    pub manager_count: Option<usize>,
    #[structopt(
        short,
        long,
        multiple = true,
        about = "list of addresses, where nsq lookup daemons can be queried, e.g. http://127.0.0.1:4161"
    )]
    pub nsqlookup: Vec<String>,
    #[structopt(
        short = "l",
        long = "listen",
        about = "address, to which server should bind",
        default_value = "127.0.0.1:8080"
    )]
    pub bind_addr: String,
}
