use std::collections::HashSet;

use actix::{Actor, Addr, Arbiter, AsyncContext, Context, StreamHandler, Supervised, Supervisor};

use crate::message::{AccountUpdatedMessage, PubSubAccount};
use crate::{manager::SubscriptionsRouter, message::SlotUpdatedMessage};
use futures::stream;
use tokio_nsq::*;

/// Actor, which is responsible for listening to the nsq messages,
/// and forward them to subscription managers, after deserialization
pub struct PubSubListner {
    /// Router, that distributes messages between `SubscriptionManager`s
    router: Addr<SubscriptionsRouter>,
    nsqlookupd: HashSet<String>,
}

impl Actor for PubSubListner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // every time this actor is restarted, resubscribe to
        // account and slot topics
        let pubsub_account_state =
            PubSubState::new("accounts", "accounts", self.nsqlookupd.clone());
        let pubsub_slot_state = PubSubState::new("slots", "slots", self.nsqlookupd.clone());
        let pubsub_accounts_stream = stream::unfold(pubsub_account_state, pubsub_accounts_listen);
        let pubsub_slot_stream = stream::unfold(pubsub_slot_state, pubsub_slots_listen);

        // re-register streams
        ctx.add_stream(pubsub_accounts_stream);
        ctx.add_stream(pubsub_slot_stream);
    }
}

impl PubSubListner {
    pub fn new(router: Addr<SubscriptionsRouter>, nsqlookupd: HashSet<String>) -> Addr<Self> {
        let listener = Self { router, nsqlookupd };
        let arbiter = Arbiter::new().handle();
        Supervisor::start_in_arbiter(&arbiter, |_| listener)
    }
}

impl Supervised for PubSubListner {
    fn restarting(&mut self, _: &mut Self::Context) {
        println!("restarting pubsub listener");
    }
}

impl StreamHandler<AccountUpdatedMessage> for PubSubListner {
    fn handle(&mut self, item: AccountUpdatedMessage, _: &mut Self::Context) {
        self.router.do_send(item);
    }
}

impl StreamHandler<SlotUpdatedMessage> for PubSubListner {
    fn handle(&mut self, item: SlotUpdatedMessage, _: &mut Self::Context) {
        self.router.do_send(item);
    }
}

/// Async function, that should be used in stream generator,
/// to produce new account updates
pub async fn pubsub_accounts_listen(
    mut state: PubSubState,
) -> Option<(AccountUpdatedMessage, PubSubState)> {
    loop {
        let message = state.consume().await?;
        let account: PubSubAccount = match serde_json::from_slice(&message.body) {
            Ok(v) => v,
            Err(e) => {
                println!("failed to deserialize account data from pubsub: {}", e);
                // notify nsq to remove message anyway, so we don't get it again
                message.finish();
                continue;
            }
        };

        let account = AccountUpdatedMessage::from(account);
        break Some((account, state));
    }
}

/// Async function, that should be used in stream generator,
/// to produce new slot upadates
pub async fn pubsub_slots_listen(
    mut state: PubSubState,
) -> Option<(SlotUpdatedMessage, PubSubState)> {
    loop {
        let message = state.consume().await?;
        let slot: SlotUpdatedMessage = match serde_json::from_slice(&message.body) {
            Ok(v) => v,
            Err(e) => {
                println!("failed to deserialize slot data from pubsub: {}", e);
                // notify nsq to remove message anyway, so we don't get it again
                message.finish();
                continue;
            }
        };
        break Some((slot, state));
    }
}

pub struct PubSubState(NSQConsumer);

impl PubSubState {
    /// Create new instance of NSQ Consumer.
    /// Arguments:
    /// * `topic`: NSQ topic to subscribe to
    /// * `channel`: NSQ channel to join, after topic subscription
    /// * `lookup`: list of NSQ lookup daemon addresses, like http://127.0.0.1:4161
    pub fn new<T: Into<String>>(topic: T, channel: T, lookup: HashSet<String>) -> Self {
        let topic = NSQTopic::new(topic).unwrap();
        let channel = NSQChannel::new(channel).unwrap();

        let consumer = NSQConsumerConfig::new(topic, channel)
            .set_sources(NSQConsumerConfigSources::Lookup(
                NSQConsumerLookupConfig::new().set_addresses(lookup),
            ))
            .build();

        Self(consumer)
    }

    /// Wrapper method to consume next nsq message, ignoring other nsq events
    #[inline]
    async fn consume(&mut self) -> Option<NSQMessage> {
        self.0.consume_filtered().await
    }
}
