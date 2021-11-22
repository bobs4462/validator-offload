use actix::{Actor, Addr, AsyncContext, Context, StreamHandler};

use crate::manager::SubscriptionManager;
use futures::stream;

pub struct PubSubListner {
    manager: Addr<SubscriptionManager>,
}

impl Actor for PubSubListner {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let pubsub_stream = stream::unfold(0, pubsub_accounts_listen);

        ctx.add_stream(pubsub_stream);
    }
}

impl PubSubListner {
    pub fn new(manager: Addr<SubscriptionManager>) -> Self {
        Self { manager }
    }
}

use crate::message::{AccountUpdatedMessage, PubSubAccount};
use tokio_nsq::*;

pub async fn pubsub_accounts_listen(mut count: u64) -> Option<(AccountUpdatedMessage, u64)> {
    let topic = NSQTopic::new("accounts").unwrap();
    let channel = NSQChannel::new("only").unwrap();

    let mut addresses = std::collections::HashSet::new();
    addresses.insert("http://127.0.0.1:4161".to_string());

    let mut consumer = NSQConsumerConfig::new(topic, channel)
        .set_max_in_flight(15)
        .set_sources(NSQConsumerConfigSources::Lookup(
            NSQConsumerLookupConfig::new().set_addresses(addresses),
        ))
        .build();

    let message = consumer.consume_filtered().await.unwrap();

    let account: PubSubAccount = serde_json::from_slice(&message.body).unwrap();
    message.finish();
    let account = AccountUpdatedMessage::from(account);
    count += 1;
    Some((account, count))
}

impl StreamHandler<AccountUpdatedMessage> for PubSubListner {
    fn handle(&mut self, item: AccountUpdatedMessage, _: &mut Self::Context) {
        self.manager.do_send(item);
    }
}
