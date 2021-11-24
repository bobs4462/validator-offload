use actix::{Message, Recipient};
use bytes::Bytes;
use serde::Deserialize;

use crate::{Pubkey, Slot, SubID, SubKey, SubscriptionKind};

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct AccountUpdatedMessage {
    pub key: SubKey,
    pub info: AccountInfo,
    pub sub: SubID,
}

#[derive(Message, Clone, Deserialize)]
#[rtype(result = "()")]
pub struct SlotUpdatedMessage {
    pub slot: Slot,
    pub parent: Slot,
}

#[derive(Clone)]
pub struct AccountInfo {
    pub lamports: u64,    // number of lamports assigned to this account
    pub owner: Pubkey,    // Pubkey of the program this account has been assigned to
    pub data: Bytes,      // data associated with the account
    pub executable: bool, // boolean indicating if the account contains a program
    pub rent_epoch: u64,  // the epoch at which this account will next owe rent
    pub slot: Slot,
}

#[derive(Message)]
#[rtype(result = "()")]
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

#[derive(Deserialize, Message, Clone)]
#[rtype(result = "()")]
pub struct PubSubAccount {
    pub pubkey: Pubkey,
    pub owner: Pubkey,
    lamports: u64,
    data: Bytes,
    rent_epoch: u64,
    executable: bool,
    pub slot: Slot,
    pub slot_status: u8,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PubSubAccountWithSubKind {
    pub account: PubSubAccount,
    pub kind: SubscriptionKind,
}

impl PubSubAccountWithSubKind {
    pub fn new(account: PubSubAccount, kind: SubscriptionKind) -> Self {
        Self { account, kind }
    }
}

impl From<PubSubAccountWithSubKind> for AccountUpdatedMessage {
    fn from(acc: PubSubAccountWithSubKind) -> Self {
        let key = SubKey::from(&acc);
        let info = AccountInfo::from(acc.account);
        let sub = SubID::default();

        Self { key, info, sub }
    }
}

impl From<PubSubAccount> for AccountInfo {
    fn from(acc: PubSubAccount) -> Self {
        AccountInfo {
            lamports: acc.lamports,
            owner: acc.owner,
            data: acc.data,
            executable: acc.executable,
            rent_epoch: acc.rent_epoch,
            slot: acc.slot,
        }
    }
}
