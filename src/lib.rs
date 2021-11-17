use serde::Deserialize;

pub mod listener;
pub mod manager;
pub mod message;
pub mod notification;
pub mod ws;

#[derive(PartialEq, Eq, Hash)]
enum SubscriptionKind {
    Acccount,
    Program,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
enum Commitment {
    Processed,
    Confirmed,
    Finalized,
}

const KEY_LEN: usize = 32;

type SubID = usize;
type Slot = u64;

type Pubkey = [u8; KEY_LEN];

#[derive(PartialEq, Eq, Hash)]
pub struct SubKey {
    key: Pubkey,
    commitment: Commitment,
    kind: SubscriptionKind,
}
