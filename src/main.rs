use structopt::StructOpt;
use ws_server::cli::CliOptions;
use ws_server::listener::PubSubListner;
use ws_server::manager::SubscriptionsRouter;
use ws_server::server::{Server, ServerState};

#[actix::main]
async fn main() -> std::io::Result<()> {
    let opts = CliOptions::from_args();
    let cores = num_cpus::get();
    let workers = opts.worker_count.unwrap_or(cores / 2);
    let managers = opts.manager_count.unwrap_or(cores / 2 - 2);

    let router = SubscriptionsRouter::new(managers);

    let state = ServerState::new(router.clone());
    let server = Server::new(state, opts.bind_addr, workers);

    PubSubListner::new(router, opts.nsqlookup.into_iter().collect());

    server.run().await?;

    Ok(())
}
