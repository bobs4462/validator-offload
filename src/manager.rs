use actix::{Actor, AsyncContext, Context, Handler, Recipient};
use std::collections::{HashMap, HashSet};

use crate::{
    message::{AccountUpdatedMessage, SlotUpdatedMessage, SubscribeMessage},
    SubKey, SubscriptionKind,
};

#[derive(Default)]
pub struct SubscriptionManager {
    account_subscriptions: HashMap<SubKey, HashSet<Recipient<AccountUpdatedMessage>>>,
    slot_subscriptions: HashSet<Recipient<SlotUpdatedMessage>>,
}

impl Actor for SubscriptionManager {
    type Context = Context<Self>;
}

impl Handler<SubscribeMessage> for SubscriptionManager {
    type Result = ();

    fn handle(&mut self, msg: SubscribeMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SubscribeMessage::AccountSubscribe(info) => {
                self.account_subscriptions
                    .entry(info.key)
                    .or_insert_with(HashSet::new)
                    .insert(info.recipient);
            }
            SubscribeMessage::SlotSubscribe(recipient) => {
                self.slot_subscriptions.insert(recipient);
            }
            SubscribeMessage::AccountUnsubscribe(info) => {
                let mut empty = false;
                if let Some(recipients) = self.account_subscriptions.get_mut(&info.key) {
                    recipients.remove(&info.recipient);
                    empty = recipients.is_empty();
                }
                if empty {
                    self.account_subscriptions.remove(&info.key);
                }
            }
            SubscribeMessage::SlotUnsubscribe(recipient) => {
                self.slot_subscriptions.remove(&recipient);
            }
        }
    }
}

impl Handler<AccountUpdatedMessage> for SubscriptionManager {
    type Result = ();

    fn handle(&mut self, msg: AccountUpdatedMessage, _: &mut Self::Context) -> Self::Result {
        let mut key = msg.key.clone();
        if let Some(recipients) = self.account_subscriptions.get(&key) {
            for r in recipients {
                let _ = r.do_send(msg.clone());
            }
        }

        key.kind = SubscriptionKind::Program;
        if let Some(recipients) = self.account_subscriptions.get(&key) {
            for r in recipients {
                let _ = r.do_send(msg.clone());
            }
        }
    }
}
