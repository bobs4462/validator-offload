use std::time::Duration;

use crate::{
    error::{SubError, SubErrorKind},
    manager::SubscriptionManager,
    message::{AccountUpdatedMessage, SlotUpdatedMessage, SubscribeMessage, SubscriptionInfo},
    notification::{AccountNotification, SlotNotification},
    subscription::{Method, PubkeyParams, SubRequest, SubResponse, SubResponseError, SubResult},
    types::SubscriptionsMap,
    SubID, SubKey, SubscriptionKind,
};
use actix::{clock::Instant, Actor, ActorContext, Addr, AsyncContext, Handler, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

/// Websocket session manager, which is responsible for
/// keeping connection alive and for servicing all
/// subscriptions, which are sent via this connection
pub struct WsSession {
    /// last heartbeat instant
    hb: Instant,
    /// address of subscription manager
    manager: Addr<SubscriptionManager>,
    /// list of subscriptions which are tracked by this
    /// session
    subscriptions: SubscriptionsMap,

    /// next subscription id to issued to client, on next
    /// subscription
    next: SubID,
    /// id of session itself
    id: u64,
}

type Success = (SubResult, u64);
type Failure = (SubError, Option<u64>);
impl WsSession {
    pub fn new(manager: Addr<SubscriptionManager>, id: u64) -> Self {
        Self {
            hb: Instant::now(),
            manager,
            subscriptions: SubscriptionsMap::default(),
            next: 0,
            id,
        }
    }

    /// Helper method, to perform regular heartbeat health
    /// checks. Will abort connection if client fails to
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

    fn process<T: AsRef<str>>(
        &mut self,
        msg: T,
        ctx: &mut WebsocketContext<Self>,
    ) -> Result<Success, Failure> {
        let request: SubRequest = match serde_json::from_str(msg.as_ref()) {
            Ok(val) => val,
            Err(e) => {
                println!("Invalid websocket message, cannot deserialize: {}", e);
                return Err((e.into(), None));
            }
        };
        use Method::*;
        match request.method {
            method @ (AccountSubscribe | ProgramSubscribe) => {
                let params = request.params.sub();
                if params.is_none() {
                    println!(
                        "Subscription parameters are invalid for request: {:?}",
                        method
                    );
                    let err = SubError::new(
                        "Invalid params: expected [<pubkey | string>, <options: map>]".into(),
                        SubErrorKind::InvalidParams,
                    );
                    return Err((err, Some(request.id)));
                }
                let PubkeyParams { pubkey, options } = params.unwrap();
                let kind = match method {
                    AccountSubscribe => SubscriptionKind::Account,
                    _ => SubscriptionKind::Program, // guaranteed to be ProgramSubscribe
                };
                let key = SubKey {
                    key: pubkey,
                    commitment: options.commitment,
                    kind,
                };
                if let Some(&id) = self.subscriptions.get_by_key(&key) {
                    return Ok((SubResult::Id(id), request.id));
                };
                let recipient = ctx.address().recipient();

                let info = SubscriptionInfo {
                    key: key.clone(),
                    recipient,
                };
                self.manager
                    .do_send(SubscribeMessage::AccountSubscribe(info));
                let id = self.next();
                self.subscriptions.insert(key, id);
                Ok((SubResult::Id(id), request.id))
            }
            method @ (AccountUnsubscribe | ProgramUnsubscribe) => {
                let params = request.params.unsub();
                if params.is_none() {
                    println!(
                        "Subscription parameters are invalid for request: {:?}",
                        method
                    );
                    let err = SubError::new(
                        "Invalid params: expected [<id | u64>]".into(),
                        SubErrorKind::InvalidParams,
                    );
                    return Err((err, Some(request.id)));
                }
                let id = params.unwrap();
                let key = self.subscriptions.remove_by_id(&id);
                if let Some(key) = key {
                    let recipient = ctx.address().recipient();

                    let info = SubscriptionInfo { key, recipient };
                    self.manager
                        .do_send(SubscribeMessage::AccountUnsubscribe(info));
                    Ok((SubResult::Status(true), request.id))
                } else {
                    let err = SubError::new(
                        "Invalid subscription id".into(),
                        SubErrorKind::InvalidParams,
                    );
                    return Err((err, Some(request.id)));
                }
            }
            method @ (SlotSubscribe | SlotUnsubscribe) => {
                let recipient = ctx.address().recipient();
                let (message, result) = match method {
                    SlotSubscribe => (
                        SubscribeMessage::SlotSubscribe(recipient),
                        SubResult::Id(self.next()),
                    ),
                    // guaranteed to be SlotUnsubscribe
                    _ => (
                        SubscribeMessage::SlotUnsubscribe(recipient),
                        SubResult::Status(true),
                    ),
                };
                self.manager.do_send(message);
                Ok((result, request.id))
            }
        }
    }
    fn next(&mut self) -> u64 {
        let id = self.next;
        self.next += 1;
        id
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
        self.hb = Instant::now();
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => self.hb = Instant::now(),
            ws::Message::Binary(bin) => {
                println!("Unexpected binary websocket message of len: {}", bin.len())
            }
            ws::Message::Text(text) => match self.process(text, ctx) {
                Ok((result, id)) => {
                    let response = SubResponse::new(id, result);
                    let response = serde_json::to_string(&response).unwrap();
                    ctx.text(response);
                }
                Err((error, id)) => {
                    let error = SubResponseError::new(id, error);
                    let error = serde_json::to_string(&error).unwrap();
                    ctx.text(error);
                }
            },
            // TODO, not sure if we even should handle those, as subscribe messages never
            // come even close to default 64KB size of websocket frames, used by awc
            ws::Message::Continuation(_) => {}
            ws::Message::Close(reason) => {
                println!("Terminating websocket connection, reason: {:?}", reason);
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}
