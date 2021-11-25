use serde::Serialize;

use crate::{
    message::{AccountInfo, AccountUpdatedMessage, SlotUpdatedMessage},
    Slot, SubID, SubscriptionKind, JSONRPC,
};

/// Notification sent over websocket connection,
/// indicating that account has changed
#[derive(Serialize)]
pub struct AccountNotification {
    jsonrpc: &'static str,
    method: &'static str,
    params: AccountNotificationParams,
}

/// Parameters of notification, contains account information
/// and client issued subscription identifier
#[derive(Serialize)]
struct AccountNotificationParams {
    result: AccountNotificationResult,
    subscription: SubID,
}

#[derive(Serialize)]
struct AccountNotificationResult {
    context: AccountNotificationContext,
    value: AccountNotificationValue,
}

#[derive(Serialize)]
struct AccountNotificationContext {
    slot: Slot,
}

/// Indicates the type of notification, can be either
/// single account related or program accounts related
#[derive(Serialize)]
#[serde(untagged)]
pub enum AccountNotificationValue {
    /// Notification for program subscription, one
    /// of the accounts of which has been updated
    Program(ProgramValue),
    /// Notification for single account update
    Account(AccountValue),
}

/// Updated account state sent as payload of notification
#[derive(Serialize)]
pub struct AccountValue {
    data: [String; 2],
    owner: String,
    rent_epoch: u64,
    lamports: u64,
    executable: bool,
}

/// Updated account state for program subscriptions, contains
/// additional public key, to indicate which account has changed
#[derive(Serialize)]
pub struct ProgramValue {
    pubkey: String,
    account: AccountValue,
}

impl From<AccountUpdatedMessage> for AccountNotification {
    fn from(msg: AccountUpdatedMessage) -> Self {
        let method = match msg.key.kind {
            SubscriptionKind::Program => "programNotification",
            SubscriptionKind::Account => "accountNotification",
        };
        let subscription = msg.sub;
        let result = AccountNotificationResult::from(msg);
        let params = AccountNotificationParams {
            result,
            subscription,
        };

        Self {
            jsonrpc: JSONRPC,
            method,
            params,
        }
    }
}

impl From<AccountUpdatedMessage> for AccountNotificationResult {
    fn from(msg: AccountUpdatedMessage) -> Self {
        let context = AccountNotificationContext {
            slot: msg.info.slot,
        };

        let value = AccountNotificationValue::from(msg);

        Self { context, value }
    }
}

impl From<AccountUpdatedMessage> for AccountNotificationValue {
    fn from(msg: AccountUpdatedMessage) -> Self {
        let account = AccountValue::from(msg.info);

        match msg.key.kind {
            SubscriptionKind::Program => {
                let pubkey = bs58::encode(msg.key.key).into_string();
                let value = ProgramValue { pubkey, account };
                Self::Program(value)
            }
            SubscriptionKind::Account => Self::Account(account),
        }
    }
}

impl From<AccountInfo> for AccountValue {
    fn from(info: AccountInfo) -> Self {
        let AccountInfo {
            data,
            owner,
            rent_epoch,
            lamports,
            executable,
            ..
        } = info;

        let data = base64::encode(
            zstd::encode_all(&data[..], 0).expect("Account data cannot be compressed"),
        );
        let data = [data, "base64+zstd".into()];
        let owner = bs58::encode(owner).into_string();

        Self {
            data,
            owner,
            rent_epoch,
            lamports,
            executable,
        }
    }
}

/// Notification indicating that slot has been updated
#[derive(Serialize)]
pub struct SlotNotification {
    jsonrpc: &'static str,
    method: &'static str,
    params: SlotNotificationParams,
}

/// Parameters of notification, contains slot information
/// and client issued subscription identifier
#[derive(Serialize)]
pub struct SlotNotificationParams {
    result: SlotNotificationResult,
    subscription: SubID,
}

/// Main payload of slot notification, contains slot
/// number and its parent slot
#[derive(Serialize)]
pub struct SlotNotificationResult {
    slot: Slot,
    parent: Slot,
}

impl From<SlotUpdatedMessage> for SlotNotification {
    fn from(msg: SlotUpdatedMessage) -> Self {
        let result = SlotNotificationResult {
            slot: msg.slot,
            parent: msg.parent,
        };
        let params = SlotNotificationParams {
            result,
            // 0, because it's not important, as only one slot subscription
            // can exist per websocket session
            subscription: 0,
        };
        Self {
            jsonrpc: JSONRPC,
            method: "slotNotification",
            params,
        }
    }
}
