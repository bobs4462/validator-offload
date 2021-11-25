use actix::{Message, Recipient};
use bytes::Bytes;
use serde::Deserialize;

use crate::{Commitment, Pubkey, Slot, SubID, SubKey, SubscriptionKind};

/// Message that contains information about which account was
/// updated, and what subscriptions it can be published to
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct AccountUpdatedMessage {
    /// Unique internal subscription identifier
    pub key: SubKey,
    /// All information about account, which will be published to client
    pub info: AccountInfo,
    /// Subscription ID issued to client, to differentiate between
    /// different notifications sent via websocket connection
    pub sub: SubID,
}

/// Message containing information about slot updates
#[derive(Message, Clone, Deserialize)]
#[rtype(result = "()")]
pub struct SlotUpdatedMessage {
    /// Slot number
    pub slot: Slot,
    /// Slot number which is considered to parent of current slot
    pub parent: Slot,
    /// Level of finalization of given slot
    pub commitment: Commitment,
}

/// Representation of account state
#[derive(Clone)]
pub struct AccountInfo {
    /// Number of lamports assigned to this account
    pub lamports: u64,
    /// Pubkey of the program this account has been assigned to
    pub owner: Pubkey,
    /// Data associated with the account
    pub data: Bytes,
    /// boolean indicating if the account contains a program
    pub executable: bool,
    /// The epoch at which this account will next owe a rent
    pub rent_epoch: u64,
    /// Slot number at which the update to account took place
    pub slot: Slot,
}

/// Message which represents various supported client
/// requests sent over websocket connection
#[derive(Message)]
#[rtype(result = "()")]
pub enum SubscribeMessage {
    /// Request to subscribe for particular account or group of program accounts
    AccountSubscribe(SubscriptionInfo),
    /// Request to subscribe to slot updates
    SlotSubscribe(Recipient<SlotUpdatedMessage>),
    /// Request to remove active account subscription
    AccountUnsubscribe(SubscriptionInfo),
    /// Request to remove active slot subscription
    SlotUnsubscribe(Recipient<SlotUpdatedMessage>),
}

/// Data describing all the pieces of information
/// necessary to create or remove subscription
pub struct SubscriptionInfo {
    /// Unique identifier of subscription
    pub key: SubKey,
    /// Address of websocket session manager to which
    /// the account updates should be sent to
    pub recipient: Recipient<AccountUpdatedMessage>,
}

/// Account update received over NSQ channel
#[derive(Deserialize, Message, Clone)]
#[rtype(result = "()")]
pub struct PubSubAccount {
    /// Public key of given account
    pub pubkey: Pubkey,
    /// Public key owner program (if any) of given account
    pub owner: Pubkey,
    lamports: u64,
    data: Bytes,
    rent_epoch: u64,
    executable: bool,
    /// Slot number at which the update was generated
    pub slot: Slot,
    /// Level of slot finalization
    pub slot_status: u8,
}

/// Wrapper type, to conveniently handling account updates which can
/// be related to both single account or program accounts subscription
#[derive(Message)]
#[rtype(result = "()")]
pub struct PubSubAccountWithSubKind {
    /// Account data
    pub account: PubSubAccount,
    /// Which kind of subscription this data should be checked against
    pub kind: SubscriptionKind,
}

impl PubSubAccountWithSubKind {
    /// Helper method to crate new instance of `PubSubAccountWithSubKind` message
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
