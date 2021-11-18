use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    manager::SubscriptionManager,
    message::{AccountUpdatedMessage, SlotUpdatedMessage, SubscribeMessage, SubscriptionInfo},
    notification::{AccountNotification, SlotNotification},
    subscription::SubRequest,
    SubID, SubKey,
};
use actix::{clock::Instant, Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

/// Websocket session manager, which is responsible for
/// keeping connection alive and for servicing all
/// subscriptions, which are sent via this connection
struct WsSession {
    /// last heartbeat instant
    hb: Instant,
    /// address of subscription manager
    manager: Arc<Addr<SubscriptionManager>>,
    /// list of subscriptions which are tracked by this
    /// session
    subscriptions: HashMap<SubKey, SubID>,
    /// next subscription id to issued to client, on next
    /// subscription
    next: SubID,
    /// id of session itself
    id: u64,
}

impl WsSession {
    fn new(manager: Arc<Addr<SubscriptionManager>>, id: u64) -> Self {
        Self {
            hb: Instant::now(),
            manager,
            subscriptions: HashMap::default(),
            next: 0,
            id,
        }
    }

    /// Helper method, to perform regular heartbeat health
    /// checks, will abort connection if client fails to
    /// respond during allowed time window
    fn hb(&self, ctx: &mut WebsocketContext<Self>) {
        let callback = |actor: &mut Self, ctx: &mut WebsocketContext<Self>| {
            let now = Instant::now();

            if now.duration_since(actor.hb) > CLIENT_TIMEOUT {
                println!("Client timed out, aborting connection");
                ctx.stop();
                return;
            }
            ctx.ping(b"PING");
        };
        ctx.run_interval(HEARTBEAT_INTERVAL, callback);
    }

    fn process<T: AsRef<str>>(&mut self, msg: T) {
        let subscription: SubRequest = match serde_json::from_str(msg.as_ref()) {
            Ok(val) => val,
            Err(e) => {
                //log the error and silently ignore invalid message
                println!("Invalid websocket message, cannot deserialize: {}", e);
                return;
            }
        };
    }
}

impl Actor for WsSession {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Initiated websocket connection #{}", self.id);
        self.hb(ctx);
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::Running {
        println!("Aborting websocket connection #{}", self.id);
        for (key, _) in self.subscriptions.drain() {
            let recipient = ctx.address().recipient();
            let info = SubscriptionInfo { key, recipient };
            let msg = SubscribeMessage::AccountUnsubscribe(info);
            self.manager.do_send(msg);
        }
        let recipient = ctx.address().recipient();
        let msg = SubscribeMessage::SlotUnsubscribe(recipient);
        self.manager.do_send(msg);

        actix::Running::Stop
    }
}

impl Handler<AccountUpdatedMessage> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: AccountUpdatedMessage, ctx: &mut Self::Context) -> Self::Result {
        let msg = AccountNotification::from(msg);
        let msg = serde_json::to_string(&msg).unwrap();
        ctx.text(msg);
    }
}

impl Handler<SlotUpdatedMessage> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: SlotUpdatedMessage, ctx: &mut Self::Context) -> Self::Result {
        let msg = SlotNotification::from(msg);
        let msg = serde_json::to_string(&msg).unwrap();
        ctx.text(msg);
    }
}

type WsMessage = Result<ws::Message, ws::ProtocolError>;
impl StreamHandler<WsMessage> for WsSession {
    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                println!(
                    "Websocket protocol error occured: {}\nAborting connection #{}",
                    e, self.id
                );
                ctx.stop();
                return;
            }
        };
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Binary(bin) => {
                println!("Unexpected binary websocket message of len: {}", bin.len())
            }
            ws::Message::Text(text) => self.process(text),
            // TODO, not sure if even should handle those, as subscribe messages never come
            // even close to default 64KB size of websocket frames, used by awc
            ws::Message::Continuation(_) => {}
            ws::Message::Close(reason) => {
                println!("Terminating websocket connection, reason: {:?}", reason);
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
