use serde::Deserialize;

pub mod listener;
pub mod manager;
pub mod message;
pub mod notification;
pub mod subscription;
pub mod ws;

#[derive(PartialEq, Eq, Hash)]
enum SubscriptionKind {
    Account,
    Program,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(Debug))]
enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

impl Default for Commitment {
    fn default() -> Self {
        Commitment::Finalized
    }
}

const KEY_LEN: usize = 32;

type SubID = u64;
type Slot = u64;

type Pubkey = [u8; KEY_LEN];

#[derive(PartialEq, Eq, Hash)]
pub struct SubKey {
    key: Pubkey,
    commitment: Commitment,
    kind: SubscriptionKind,
}

const JSONRPC: &str = "2.0";
