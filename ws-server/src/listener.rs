use std::collections::HashSet;

use actix::{Actor, Addr, Arbiter, AsyncContext, Context, StreamHandler, Supervised, Supervisor};
use futures::stream;
use rmp_serde as rmps;
use tokio_nsq::*;

use crate::message::PubSubAccount;
use crate::{manager::SubscriptionsRouter, message::SlotUpdatedMessage};
use crate::{Slot, METRICS};

/// Actor, which is responsible for listening to the NSQ messages,
/// and forward them to subscription managers, after deserialization
pub struct PubSubListner {
    /// Router, that distributes messages between `SubscriptionManager`s
    router: Addr<SubscriptionsRouter>,
    /// List of web addresses, which can be used to locate NSQ lookup deamons in network
    nsqlookupd: HashSet<String>,
    /// Largest slot number, observed from pubsub
    max_slot: Slot,
}

impl Actor for PubSubListner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // every time this actor is restarted, resubscribe to
        // account and slot topics all over again
        let pubsub_account_state =
            PubSubState::new("accounts", "accounts", self.nsqlookupd.clone());
        let pubsub_slot_state = PubSubState::new("slots", "slots", self.nsqlookupd.clone());
        let pubsub_accounts_stream = stream::unfold(pubsub_account_state, pubsub_accounts_listen);
        let pubsub_slot_stream = stream::unfold(pubsub_slot_state, pubsub_slots_listen);

        // re-register streams
        ctx.add_stream(pubsub_accounts_stream);
        ctx.add_stream(pubsub_slot_stream);
        println!("Subscribed to NSQ pubsub topics");
    }
}

impl PubSubListner {
    /// Create a new listener
    pub fn new(router: Addr<SubscriptionsRouter>, nsqlookupd: HashSet<String>) -> Addr<Self> {
        let listener = Self {
            router,
            nsqlookupd,
            max_slot: 0,
        };
        let arbiter = Arbiter::new().handle();
        Supervisor::start_in_arbiter(&arbiter, |_| listener)
    }
}

impl Supervised for PubSubListner {
    fn restarting(&mut self, _: &mut Self::Context) {
        println!("restarting pubsub listener");
    }
}

impl StreamHandler<PubSubAccount> for PubSubListner {
    fn handle(&mut self, item: PubSubAccount, _: &mut Self::Context) {
        METRICS.account_updates_count.inc();
        self.router.do_send(item);
    }
}

impl StreamHandler<SlotUpdatedMessage> for PubSubListner {
    fn handle(&mut self, item: SlotUpdatedMessage, _: &mut Self::Context) {
        println!("Got slot");
        METRICS.account_updates_count.inc();

        self.max_slot = self.max_slot.max(item.slot);
        METRICS.slot.set(self.max_slot as i64);
        self.router.do_send(item);
    }
}

/// Async function, that should be used in stream generator,
/// to produce new account updates
pub async fn pubsub_accounts_listen(
    mut state: PubSubState,
) -> Option<(PubSubAccount, PubSubState)> {
    loop {
        let message = state.consume().await?;
        let account: PubSubAccount = match rmps::from_read(message.body.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                println!("failed to deserialize account data from pubsub: {}", e);
                // notify nsq to remove message anyway, so we nsq doesn't requeue it
                message.finish();
                continue;
            }
        };
        message.finish();

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
        let slot: SlotUpdatedMessage = match rmps::from_read(message.body.as_slice()) {
            Ok(v) => v,
            Err(e) => {
                println!("failed to deserialize slot data from pubsub: {}", e);
                // notify nsq to remove message anyway, so we don't get it again
                message.finish();
                continue;
            }
        };
        message.finish();
        break Some((slot, state));
    }
}

/// Wrapping type to hold consumer of NSQ messages
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
