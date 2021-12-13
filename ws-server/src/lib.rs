#![deny(missing_docs)]
//! Websocket server for solana network
use std::hash::Hash;

use message::PubSubAccountWithSubKind;
use serde::Deserialize;

/// Handling of temporarily buffered, not yet finalized accounts
pub mod buffer;
/// Command line options, provided at application startup
pub mod cli;
/// Collection of application specific errors
pub mod error;
/// Handling of message consumption from NSQ pubsub
pub mod listener;
/// Subscription manager and subscription router to distribute work
/// among several subscription managers
pub mod manager;
/// Various messages sent between actors in application
pub mod message;
/// Various metrics, collected across different application components
mod metrics;
/// Update notifications sent to subscribed clients
pub mod notification;
/// Main entry point to run http server to accept websocket connections
pub mod server;
/// Handling of websocket session and keeping track of subscriptions
/// for this particular session
pub mod session;
/// Data structure to keep track of slot updates
mod slotree;
/// Subscription requests sent from client to server via established
/// websocket connection
pub mod subscription;
/// Components testing
mod tests;
/// Helper data structures
pub mod types;

const JSONRPC: &str = "2.0";
const KEY_LEN: usize = 32;

/// Type of subscription, can be either for single account or for all
/// accounts which are owned by specified program
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum SubscriptionKind {
    /// Subscription is for account
    Account,
    /// Subscription is accounts owned by a program
    Program,
}

/// Commitment level, indicates how storngly, a particular record,
/// is accepted by solana cluster  
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(Debug))]
pub enum Commitment {
    /// The most recent update, state at this level might be
    /// rolled back by cluster
    Processed = 1,
    /// Supermajority of the cluster has voted on this state,
    /// not likely to rolled back
    Confirmed = 2,
    /// Supermajority of the cluster has confirmed this state
    Finalized = 3,
}

/// Id of subscription, issued to client, to different
/// between various active subscriptions
type SubID = u64;
/// Slot number
type Slot = u64;

type Pubkey = [u8; KEY_LEN];

/// A key to uniquely identify a given subscription by client
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SubKey {
    key: Pubkey,
    commitment: Commitment,
    kind: SubscriptionKind,
}

impl Default for Commitment {
    fn default() -> Self {
        Commitment::Finalized
    }
}

impl From<u8> for Commitment {
    fn from(status: u8) -> Self {
        match status {
            2 => Commitment::Confirmed,
            3 => Commitment::Finalized,
            _ => Commitment::Processed,
        }
    }
}

impl Commitment {
    /// Whether the given commitment has been confirmed by supermajority
    pub fn confirmed(&self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

impl SubKey {
    /// Create a default subscription key, with given public key
    #[inline]
    pub fn new(key: Pubkey) -> Self {
        Self {
            key,
            commitment: Commitment::Processed,
            kind: SubscriptionKind::Account,
        }
    }
    /// Builder like method to change commitment level of subscription key
    #[inline]
    pub fn commitment(mut self, slot_status: u8) -> Self {
        self.commitment = slot_status.into();
        self
    }

    /// Builder like method to change subscription type of subscription key
    #[inline]
    pub fn kind(mut self, kind: SubscriptionKind) -> Self {
        self.kind = kind;
        self
    }
}

impl From<&PubSubAccountWithSubKind> for SubKey {
    fn from(acc: &PubSubAccountWithSubKind) -> Self {
        let pubkey = match acc.kind {
            SubscriptionKind::Account => acc.account.pubkey,
            SubscriptionKind::Program => acc.account.owner,
        };
        SubKey::new(pubkey)
            .commitment(acc.account.slot_status)
            .kind(acc.kind.clone())
    }
}

pub use metrics::METRICS;
