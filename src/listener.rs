use std::collections::HashSet;

use actix::{Actor, Addr, AsyncContext, Context, StreamHandler};

use crate::{manager::SubscriptionsRouter, message::SlotUpdatedMessage};
use futures::stream;

pub struct PubSubListner {
    /// Router, that distributes messages between `SubscriptionManager`s
    router: Addr<SubscriptionsRouter>,
}

impl Actor for PubSubListner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut lookup = HashSet::new();
        lookup.insert("http://127.0.0.1:4161".into());
        let pubsub_account_state = PubSubState::new("accounts", "accounts", lookup.clone());
        let pubsub_slot_state = PubSubState::new("slots", "slots", lookup.clone());
        let pubsub_accounts_stream = stream::unfold(pubsub_account_state, pubsub_accounts_listen);
        let pubsub_slot_stream = stream::unfold(pubsub_slot_state, pubsub_slots_listen);

        ctx.add_stream(pubsub_accounts_stream);
        ctx.add_stream(pubsub_slot_stream);
    }
}

impl PubSubListner {
    pub fn new(router: Addr<SubscriptionsRouter>) -> Self {
        Self { router }
    }
}

use crate::message::{AccountUpdatedMessage, PubSubAccount};
use tokio_nsq::*;

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

    #[inline]
    async fn consume(&mut self) -> Option<NSQMessage> {
        self.0.consume_filtered().await
    }
}
