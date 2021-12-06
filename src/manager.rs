use actix::{Actor, Addr, Arbiter, Context, Handler, Recipient, Supervised, Supervisor};
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::buffer::Buffer;
use crate::message::{PubSubAccountWithSubKind, SetBufferManager};
use crate::SubscriptionKind;
use crate::{
    message::{AccountUpdatedMessage, PubSubAccount, SlotUpdatedMessage, SubscribeMessage},
    SubKey,
};

/// Main struct to track which websocket sessions are interested
/// in which kinds of updates. Keeps to separate mappings to track
/// account related and slot related subscriptions respectively
pub struct SubscriptionManager {
    account_subscriptions: HashMap<SubKey, HashSet<Recipient<AccountUpdatedMessage>>>,
    slot_subscriptions: HashSet<Recipient<SlotUpdatedMessage>>,
    buffer_manager: Option<Addr<Buffer>>,
    id: usize,
}

/// Load balancer for subscriptions, evenly distributes work among
/// several `SubscriptionManager`s based on hash value of unique
/// subscription identifier: for account and program subscriptions
/// this identifier is the `SubKey`, for slot subscriptions it is
/// just the address of websocket session manager.
pub struct SubscriptionsRouter {
    // subscription managers available in the pool
    managers: Vec<Addr<SubscriptionManager>>,
    // buffer manager, to track non-finalized accounts
    buffer_manager: Option<Addr<Buffer>>,
}

impl SubscriptionManager {
    fn new(id: usize) -> Self {
        let account_subscriptions = HashMap::default();
        let slot_subscriptions = HashSet::default();
        Self {
            id,
            account_subscriptions,
            slot_subscriptions,
            buffer_manager: None,
        }
    }
}

#[cfg(test)]
impl SubscriptionManager {
    pub fn account_sub_count(&self, key: &SubKey) -> usize {
        self.account_subscriptions
            .get(&key)
            .map(|set| set.len())
            .unwrap_or_default()
    }
    pub fn slot_sub_count(&self) -> usize {
        self.slot_subscriptions.len()
    }
}

impl SubscriptionsRouter {
    /// Create a new instance of subscriptions router, with
    /// specified number of subscription managers, start all
    /// managers in separate threads as Actors, collect their
    /// addresses, and finally start self as an Actor in yet
    /// another separate thread and return its own address
    pub fn new(pool_size: usize) -> Addr<Self> {
        let mut managers = Vec::with_capacity(pool_size);
        for id in 0..pool_size {
            let sm = SubscriptionManager::new(id);
            let arbiter = Arbiter::new().handle();
            let addr = Supervisor::start_in_arbiter(&arbiter, |_| sm);
            managers.push(addr);
        }
        let router = Self {
            managers,
            buffer_manager: None,
        };
        let arbiter = Arbiter::new().handle();
        Supervisor::start_in_arbiter(&arbiter, |_| router)
    }

    /// Get the address of subscription manager, which should handle
    /// related message, based on hash value of the unique message identifier
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

impl Handler<PubSubAccountWithSubKind> for SubscriptionManager {
    type Result = ();

    fn handle(&mut self, acc: PubSubAccountWithSubKind, _: &mut Self::Context) -> Self::Result {
        let key = SubKey::from(&acc);

        if let Some(recipients) = self.account_subscriptions.get_mut(&key) {
            let pubsub_account = acc.account.clone();
            if pubsub_account.slot_status == 1 {
                // Account has been processed, start tracking it for slot
                // status update, ignore other commitment levels
                let bm = self
                    .buffer_manager
                    .as_ref()
                    .expect("No buffer manager is set up for submanager");
                bm.do_send(pubsub_account);
            }
            let update = AccountUpdatedMessage::from(acc);
            let mut failed = Vec::new();
            // Broadcast the account update to all websocket session managers,
            // which have registered themselves for it
            for r in recipients.iter() {
                if let Err(e) = r.do_send(update.clone()) {
                    println!("failed to send account data to ws session: {}", e);
                    failed.push(r.clone());
                }
            }
            // Remove inactive subscriptions, for which there's no active websocket session
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

impl Handler<PubSubAccount> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, acc: PubSubAccount, _ctx: &mut Self::Context) -> Self::Result {
        // Get address of manager by account key
        let mut key = SubKey::new(acc.pubkey).commitment(acc.slot_status);
        let mut addr = self.addr(&key);
        let mut update = PubSubAccountWithSubKind::new(acc.clone(), SubscriptionKind::Account);

        addr.do_send(update);

        // Get address of manager by account owner key, to check for program subscriptions
        key = SubKey::new(acc.owner)
            .commitment(acc.slot_status)
            .kind(SubscriptionKind::Program);
        addr = self.addr(&key);
        update = PubSubAccountWithSubKind::new(acc, SubscriptionKind::Program);
        addr.do_send(update);
    }
}

impl Handler<SlotUpdatedMessage> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, msg: SlotUpdatedMessage, _ctx: &mut Self::Context) -> Self::Result {
        // broadcast slot message to all subscription managers
        for addr in &self.managers {
            addr.do_send(msg.clone());
        }
        // also forward the slot to buffer manager, to send notifications
        // for accounts, which are related to given slot
        let bm = self
            .buffer_manager
            .as_ref()
            .expect("No buffer manager is set up for subrouter");

        bm.do_send(msg);
    }
}

impl Handler<SetBufferManager> for SubscriptionsRouter {
    type Result = ();

    fn handle(&mut self, msg: SetBufferManager, _: &mut Self::Context) -> Self::Result {
        // forward message to all subscription managers
        for m in &self.managers {
            m.do_send(msg.clone());
        }

        self.buffer_manager.replace(msg.0);
    }
}

impl Handler<SetBufferManager> for SubscriptionManager {
    type Result = ();

    fn handle(&mut self, msg: SetBufferManager, _: &mut Self::Context) -> Self::Result {
        // set buffer manager's address
        self.buffer_manager = Some(msg.0);
    }
}
