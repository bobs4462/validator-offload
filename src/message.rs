use actix::{Message, Recipient};

use crate::{Pubkey, Slot, SubID, SubKey};

#[derive(Message)]
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
