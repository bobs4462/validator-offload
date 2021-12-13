use std::collections::BTreeMap;

use actix::{Actor, Addr, Arbiter, Context, Handler, Supervised, Supervisor};

use crate::{
    manager::SubscriptionsRouter,
    message::{PubSubAccount, SlotUpdatedMessage, TrackAccount},
    slotree::SlotTree,
    Slot,
};

/// Type for buffering the non-finalized accounts, for which
/// there are exist active subscriptions. It will keep track of
/// all not finalized accounts, until they are finalized, or their
/// update branches are discarded in blockchain
pub struct Buffer {
    /// Accounts whose slots haven't been finalized yet. Every
    /// time slot is updated, all the related accounts will be
    /// sent to subscribers for the given commitment level.
    accounts: BTreeMap<Slot, Vec<PubSubAccount>>,
    /// Tracking of slot to parent relations, used for cleanup
    /// purposes, to remove orphaned slots and related accounts
    slots: SlotTree,
    /// Router, to distribute messages between `SubscriptionManager`s
    router: Addr<SubscriptionsRouter>,
}

impl Buffer {
    /// Convenient constructor, to start up buffering service in
    /// a separate thread as an Actor, and return its address
    pub fn new(router: Addr<SubscriptionsRouter>) -> Addr<Self> {
        let arbiter = Arbiter::new().handle();
        let buffer = Self {
            accounts: BTreeMap::default(),
            slots: SlotTree::new(),
            router,
        };
        Supervisor::start_in_arbiter(&arbiter, |_| buffer)
    }
}

impl Actor for Buffer {
    type Context = Context<Self>;
}

impl Supervised for Buffer {
    fn restarting(&mut self, _: &mut Self::Context) {
        println!("restarting account's buffering handler");
    }
}

impl Handler<PubSubAccount> for Buffer {
    type Result = ();

    fn handle(&mut self, acc: PubSubAccount, _: &mut Self::Context) -> Self::Result {
        self.accounts.entry(acc.slot).or_default().push(acc);
    }
}

impl Handler<SlotUpdatedMessage> for Buffer {
    type Result = ();

    fn handle(&mut self, update: SlotUpdatedMessage, _: &mut Self::Context) -> Self::Result {
        if update.status.confirmed() {
            let accounts = self.accounts.get_mut(&update.slot).into_iter().flatten();
            for mut acc in accounts {
                acc.slot_status = 2; // confirmed slot
                self.router.do_send(acc.clone());
            }
        }
        let rooted_or_pruned = self.slots.push(update);

        // if slot status wasn't rooted, none of the code below will be executed
        for slot in rooted_or_pruned.into_iter().flatten() {
            let accounts = self.accounts.remove(&slot).into_iter().flatten();
            if slot.rooted() {
                for mut acc in accounts {
                    acc.slot_status = 3; // finalized slot
                    self.router.do_send(acc);
                }
            } // else slot has been pruned, so we just drop related accounts
        }
        // remove dead slots: which weren't rooted or pruned
        let root = self.slots.current_root();
        let dead: Vec<Slot> = self
            .accounts
            .range(..root)
            .map(|(k, _)| k)
            .copied()
            .collect();
        for slot in dead {
            self.accounts.remove(&slot);
        }
    }
}

impl Handler<TrackAccount> for Buffer {
    type Result = ();

    fn handle(&mut self, msg: TrackAccount, _: &mut Self::Context) -> Self::Result {
        self.accounts
            .entry(msg.0.slot)
            .or_insert_with(Vec::new)
            .push(msg.0);
    }
}
