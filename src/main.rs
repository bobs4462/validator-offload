use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use actix::{Actor, Addr};
use actix_web::web::{Data, HttpRequest, HttpResponse, Payload};
use actix_web::{get, App, Error as HttpError, HttpServer};
use actix_web_actors::ws;
use std::sync::atomic::Ordering;
use ws_server::listener::PubSubListner;
use ws_server::{manager::SubscriptionManager, ws::WsSession};

#[actix::main]
async fn main() -> std::io::Result<()> {
    let manager = SubscriptionManager::default().start();
    let state = State {
        manager: manager.clone(),
        next: Arc::new(AtomicU64::new(0)),
    };

    PubSubListner::new(manager).start();

    HttpServer::new(move || {
        App::new()
            .service(start_connection) //. rename with "as" import or naming conflict
            .app_data(Data::new(state.clone())) //register the lobby
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[get("/")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    state: Data<State>,
) -> Result<HttpResponse, HttpError> {
    let session = WsSession::new(state.manager.clone(), state.next.load(Ordering::Relaxed));
    state.next.fetch_add(1, Ordering::Relaxed);

    let resp = ws::start(session, &req, stream)?;
    Ok(resp)
}

#[derive(Clone)]
pub struct State {
    manager: Addr<SubscriptionManager>,
    next: Arc<AtomicU64>,
}
