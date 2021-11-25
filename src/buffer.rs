use std::collections::{BTreeMap, HashMap, HashSet};

use actix::{Actor, Addr, Arbiter, Context, Supervised, Supervisor};

use crate::{message::PubSubAccount, Pubkey, Slot};

/// Type for buffering not yet finalized accounts, for which
/// there are exist active subscriptions
#[derive(Default)]
pub struct Buffer {
    /// Accounts whose slots haven't been finalized yet. Every
    /// time slot is updated, all the related accounts will be
    /// sent to subscribers for the given commitment level.
    accounts: HashMap<Slot, Vec<PubSubAccount>>,
    /// Tracking of slot to parent relations, used for cleanup
    /// purposes, to remove orphaned slots and related accounts
    slots: BTreeMap<Slot, Slot>,
    /// Account or Program keys, which we have subscriptions for.
    /// Used for filtering purposes, so there's no need to track
    /// commitment levels for accounts, we are not interested in.
    pubkeys: HashSet<Pubkey>,
}

impl Buffer {
    /// Handy constructor, to start buffering service in a
    /// separate thread as an Actor, and return its address
    pub fn new() -> Addr<Self> {
        let arbiter = Arbiter::new().handle();
        let buffer = Self::default();
        Supervisor::start_in_arbiter(&arbiter, |_| buffer)
    }
}

impl Actor for Buffer {
    type Context = Context<Self>;
}

impl Supervised for Buffer {
    fn restarting(&mut self, _: &mut Self::Context) {
        println!("restarting cache handler");
    }
}
