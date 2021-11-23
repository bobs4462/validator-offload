use std::hash::Hash;

use serde::Deserialize;

pub mod cli;
pub mod error;
pub mod listener;
pub mod manager;
pub mod message;
pub mod notification;
pub mod server;
pub mod session;
pub mod subscription;
pub mod types;

const JSONRPC: &str = "2.0";
const KEY_LEN: usize = 32;

#[derive(PartialEq, Eq, Hash, Clone)]
enum SubscriptionKind {
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
