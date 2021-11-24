use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{message::PubSubAccount, Pubkey, Slot};

/// Type for buffering not finalized accounts, for which there
/// are exist active subscriptions
pub struct Cache {
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
