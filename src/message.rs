use actix::{Message, Recipient};

use crate::{Pubkey, Slot, SubKey};

#[derive(Message)]
#[rtype(type = "()")]
pub struct AccountUpdatedMessage {
    pub key: SubKey,
    pub info: AccountInfo,
}

#[derive(Message)]
#[rtype(type = "()")]
pub struct SlotUpdatedMessage {
    slot: u64,
    parent: u64,
    root: u64,
}

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

impl AccountUpdatedMessage {
    fn to_json_string(&self) -> String {
        "Hello".to_owned()
    }
}
