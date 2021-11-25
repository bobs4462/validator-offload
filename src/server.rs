use actix::Addr;
use actix_web::web::{Data, HttpRequest, HttpResponse, Payload};
use actix_web::{get, App, Error as HttpError, HttpServer};
use actix_web_actors::ws;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::manager::SubscriptionsRouter;
use crate::session::WsSession;

#[get("/")]
pub async fn connect(
    req: HttpRequest,
    stream: Payload,
    state: Data<ServerState>,
) -> Result<HttpResponse, HttpError> {
    let session = WsSession::new(state.router.clone(), state.next.load(Ordering::Relaxed));
    state.next.fetch_add(1, Ordering::Relaxed);

    let resp = ws::start(session, &req, stream)?;
    Ok(resp)
}

/// Helper type to contain state and configuration of server before running
pub struct Server {
    state: ServerState,
    addr: String,
    workers: usize,
}

impl Server {
    /// Start server process with configured parameters
    pub async fn run(self) -> std::io::Result<()> {
        HttpServer::new(move || {
            App::new()
                .service(connect)
                .app_data(Data::new(self.state.clone()))
        })
        .workers(self.workers)
        .bind(self.addr)?
        .run()
        .await?;
        Ok(())
    }
}

/// Server state, that is accessable to all worker threads.
/// Separate copy of state is provided for each worker.
#[derive(Clone)]
pub struct ServerState {
    router: Addr<SubscriptionsRouter>,
    next: Arc<AtomicU64>,
}

impl ServerState {
    /// Construct new server state
    pub fn new(router: Addr<SubscriptionsRouter>) -> Self {
        Self {
            router,
            next: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Server {
    /// Construct new server instance with given configuration
    pub fn new(state: ServerState, addr: String, workers: usize) -> Self {
        Self {
            state,
            addr,
            workers,
        }
    }
}
