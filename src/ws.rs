use std::{collections::HashMap, sync::Arc};

use crate::{manager::SubscriptionManager, message::AccountUpdatedMessage, SubID, SubKey};
use actix::{clock::Instant, Actor, Addr, Handler};
use actix_web_actors::ws::WebsocketContext;

struct WsSession {
    hb: Instant,
    managers: Arc<Vec<Addr<SubscriptionManager>>>,
    subscriptions: HashMap<SubKey, SubID>,
}

pub struct SessionManager {
    sessions: Vec<Addr<WsSession>>,
}

impl Actor for WsSession {
    type Context = WebsocketContext<Self>;
}

impl Handler<AccountUpdatedMessage> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: AccountUpdatedMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text("Hello");
    }
}
