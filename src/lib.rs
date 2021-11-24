use std::hash::Hash;

use message::PubSubAccountWithSubKind;
use serde::Deserialize;

pub mod cache;
pub mod cli;
pub mod error;
pub mod listener;
pub mod manager;
pub mod message;
mod metrics;
pub mod notification;
pub mod server;
pub mod session;
pub mod subscription;
mod tests;
pub mod types;

const JSONRPC: &str = "2.0";
const KEY_LEN: usize = 32;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum SubscriptionKind {
    Account,
    Program,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(Debug))]
pub enum Commitment {
    Processed = 1,
    Confirmed = 2,
    Finalized = 3,
}

type SubID = u64;
type Slot = u64;

type Pubkey = [u8; KEY_LEN];

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

impl SubKey {
    #[inline]
    pub fn new(key: Pubkey) -> Self {
        Self {
            key,
            commitment: Commitment::Processed,
            kind: SubscriptionKind::Account,
        }
    }
    #[inline]
    pub fn commitment(mut self, slot_status: u8) -> Self {
        self.commitment = slot_status.into();
        self
    }
    #[inline]
    pub fn kind(mut self, kind: SubscriptionKind) -> Self {
        self.kind = kind;
        self
    }
}

impl From<&PubSubAccountWithSubKind> for SubKey {
    fn from(acc: &PubSubAccountWithSubKind) -> Self {
        let pubkey = match acc.kind {
            SubscriptionKind::Account => acc.account.pubkey.clone(),
            SubscriptionKind::Program => acc.account.owner.clone(),
        };
        SubKey::new(pubkey)
            .commitment(acc.account.slot_status)
            .kind(acc.kind.clone())
    }
}

pub use metrics::METRICS;
