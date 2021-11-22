use actix::{Message, Recipient};
use serde::Deserialize;

use crate::{Commitment, Pubkey, Slot, SubID, SubKey, SubscriptionKind};

#[derive(Message, Clone)]
#[rtype(type = "()")]
pub struct AccountUpdatedMessage {
    pub key: SubKey,
    pub info: AccountInfo,
    pub sub: SubID,
}

#[derive(Message)]
#[rtype(type = "()")]
pub struct SlotUpdatedMessage {
    pub slot: u64,
    pub parent: u64,
}

#[derive(Clone)]
pub struct AccountInfo {
    pub lamports: u64,    // number of lamports assigned to this account
    pub owner: Pubkey,    // Pubkey of the program this account has been assigned to
    pub data: Vec<u8>,    // data associated with the account
    pub executable: bool, // boolean indicating if the account contains a program
    pub rent_epoch: u64,  // the epoch at which this account will next owe rent
    pub slot: Slot,
}

#[derive(Message)]
#[rtype(type = "()")]
pub enum SubscribeMessage {
    AccountSubscribe(SubscriptionInfo),
    SlotSubscribe(Recipient<SlotUpdatedMessage>),
    AccountUnsubscribe(SubscriptionInfo),
    SlotUnsubscribe(Recipient<SlotUpdatedMessage>),
}

pub struct SubscriptionInfo {
    pub key: SubKey,
    pub recipient: Recipient<AccountUpdatedMessage>,
}

#[derive(Deserialize)]
pub struct PubSubAccount {
    pub pubkey: Pubkey,
    owner: Pubkey,
    lamports: u64,
    data: Vec<u8>,
    rent_epoch: u64,
    executable: bool,
    pub slot: Slot,
    slot_status: u8,
}

impl From<PubSubAccount> for AccountUpdatedMessage {
    fn from(acc: PubSubAccount) -> Self {
        let commitment = match acc.slot_status {
            1 => Commitment::Processed,
            2 => Commitment::Confirmed,
            _ => Commitment::Finalized,
        };
        let key = SubKey {
            key: acc.pubkey,
            commitment,
            kind: SubscriptionKind::Account,
        };
        let info = AccountInfo {
            lamports: acc.lamports,
            owner: acc.owner,
            data: acc.data,
            executable: acc.executable,
            rent_epoch: acc.rent_epoch,
            slot: acc.slot,
        };
        let sub = 0;

        Self { key, info, sub }
    }
}
