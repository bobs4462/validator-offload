use actix::{Actor, Addr, Arbiter, Context, Handler, Recipient, Supervised, Supervisor};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::{
    message::{AccountUpdatedMessage, SlotUpdatedMessage, SubscribeMessage},
    SubKey,
};

#[derive(Default)]
pub struct SubscriptionManager {
    account_subscriptions: HashMap<SubKey, HashSet<Recipient<AccountUpdatedMessage>>>,
    slot_subscriptions: HashSet<Recipient<SlotUpdatedMessage>>,
    id: usize,
}

pub struct SubscriptionsRouter {
    // subscription managers available in the pool
    managers: Vec<Addr<SubscriptionManager>>,
}

impl SubscriptionsRouter {
    pub fn new(pool_size: usize) -> Addr<Self> {
        let mut managers = Vec::with_capacity(pool_size);
        for id in 0..pool_size {
            let mut sm = SubscriptionManager::default();
            sm.id = id;
            let arbiter = Arbiter::new().handle();
            let addr = Supervisor::start_in_arbiter(&arbiter, |_| sm);
            managers.push(addr);
        }
        let router = Self { managers };
        let arbiter = Arbiter::new().handle();
        Supervisor::start_in_arbiter(&arbiter, |_| router)
    }

    /// Get the address of subscription manager, which should handle related message
    #[inline]
    pub fn addr<T: Hash>(&self, item: T) -> &Addr<SubscriptionManager> {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let idx = hasher.finish() as usize % self.managers.len();
        &self.managers[idx]
    }
}

impl Actor for SubscriptionManager {
    type Context = Context<Self>;
}

impl Actor for SubscriptionsRouter {
    type Context = Context<Self>;
}

impl Supervised for SubscriptionManager {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        println!("restarting subscription manager #{}", self.id);
    }
}

impl Supervised for SubscriptionsRouter {
    fn restarting(&mut self, _ctx: &mut Self::Context) {
        println!("restarting subscription router");
    }
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
        if let Some(recipients) = self.account_subscriptions.get_mut(&msg.key) {
            let mut failed = Vec::new();
            for r in recipients.iter() {
                if let Err(e) = r.do_send(msg.clone()) {
                    println!("failed to send account data to ws session: {}", e);
                    failed.push(r.clone());
                }
            }
            for f in failed {
                recipients.remove(&f);
            }
        }
    }
}

impl Handler<SlotUpdatedMessage> for SubscriptionManager {
    type Result = ();

    fn handle(&mut self, msg: SlotUpdatedMessage, _ctx: &mut Self::Context) -> Self::Result {
        let mut failed = Vec::new();
        for r in &self.slot_subscriptions {
            if let Err(e) = r.do_send(msg.clone()) {
                println!("failed to send slot data to ws session: {}", e);
                failed.push(r.clone());
            }
        }
        for f in failed {
            self.slot_subscriptions.remove(&f);
        }
    }
}

impl Handler<SubscribeMessage> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, msg: SubscribeMessage, _ctx: &mut Self::Context) -> Self::Result {
        let addr = match msg {
            SubscribeMessage::AccountSubscribe(ref info)
            | SubscribeMessage::AccountUnsubscribe(ref info) => self.addr(&info.key),
            SubscribeMessage::SlotUnsubscribe(ref recipient)
            | SubscribeMessage::SlotSubscribe(ref recipient) => self.addr(recipient),
        };
        addr.do_send(msg);
    }
}

impl Handler<AccountUpdatedMessage> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, msg: AccountUpdatedMessage, _ctx: &mut Self::Context) -> Self::Result {
        let addr = self.addr(&msg.key);
        addr.do_send(msg);
    }
}

impl Handler<SlotUpdatedMessage> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, msg: SlotUpdatedMessage, _ctx: &mut Self::Context) -> Self::Result {
        // broadcast slot message to all subscription managers
        for addr in &self.managers {
            addr.do_send(msg.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use actix::Message;
    #[derive(Message)]
    #[rtype(result = "usize")]
    enum CountRequestMessage {
        AccountSubscriptionsCount(SubKey),
        SlotSubscriptionsCount,
    }

    #[derive(Message)]
    #[rtype(result = "Addr<SubscriptionManager>")]
    struct GetAddr<T: Hash>(T);

    impl Handler<CountRequestMessage> for SubscriptionManager {
        type Result = usize;

        fn handle(&mut self, msg: CountRequestMessage, _ctx: &mut Self::Context) -> Self::Result {
            match msg {
                CountRequestMessage::SlotSubscriptionsCount => self.slot_subscriptions.len(),
                CountRequestMessage::AccountSubscriptionsCount(key) => {
                    let subs = self.account_subscriptions.get(&key);
                    subs.unwrap_or(&HashSet::new()).len()
                }
            }
        }
    }

    impl<T: Hash> Handler<GetAddr<T>> for SubscriptionsRouter {
        type Result = Addr<SubscriptionManager>;

        fn handle(&mut self, msg: GetAddr<T>, _ctx: &mut Self::Context) -> Self::Result {
            self.addr(msg.0).clone()
        }
    }

    use super::*;
    use crate::{message::SubscriptionInfo, Commitment, SubscriptionKind};
    #[actix::test]
    async fn test_routing() {
        let router = SubscriptionsRouter::new(4);
        let subkey = SubKey {
            key: [1; 32],
            commitment: Commitment::Processed,
            kind: SubscriptionKind::Account,
        };
        let manager = SubscriptionManager::default().start();

        router.do_send(SubscribeMessage::AccountSubscribe(SubscriptionInfo {
            key: subkey.clone(),
            recipient: manager.clone().recipient(),
        }));
        let addr = router.send(GetAddr(subkey.clone())).await.unwrap();
        let mut acc_sub_count = addr
            .send(CountRequestMessage::AccountSubscriptionsCount(
                subkey.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(acc_sub_count, 1);
        router
            .send(SubscribeMessage::AccountUnsubscribe(SubscriptionInfo {
                key: subkey.clone(),
                recipient: manager.clone().recipient(),
            }))
            .await
            .unwrap();
        acc_sub_count = addr
            .send(CountRequestMessage::AccountSubscriptionsCount(
                subkey.clone(),
            ))
            .await
            .unwrap();
        assert_eq!(acc_sub_count, 0);

        router.do_send(SubscribeMessage::SlotSubscribe(manager.clone().recipient()));
        let addr = router
            .send(GetAddr(manager.clone().recipient::<SlotUpdatedMessage>()))
            .await
            .unwrap();
        let mut slot_sub_count = addr
            .send(CountRequestMessage::SlotSubscriptionsCount)
            .await
            .unwrap();
        assert_eq!(slot_sub_count, 1);
        router
            .send(SubscribeMessage::SlotUnsubscribe(manager.recipient()))
            .await
            .unwrap();
        slot_sub_count = addr
            .send(CountRequestMessage::SlotSubscriptionsCount)
            .await
            .unwrap();
        assert_eq!(slot_sub_count, 0);
    }
}
