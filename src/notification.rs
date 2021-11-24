use serde::Serialize;

use crate::{
    message::{AccountInfo, AccountUpdatedMessage, SlotUpdatedMessage},
    Slot, SubID, SubscriptionKind, JSONRPC,
};

#[derive(Serialize)]
pub struct AccountNotification {
    jsonrpc: &'static str,
    method: &'static str,
    params: AccountNotificationParams,
}

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

#[derive(Serialize)]
#[serde(untagged)]
pub enum AccountNotificationValue {
    Program(ProgramValue),
    Account(AccountValue),
}

#[derive(Serialize)]
pub struct AccountValue {
    data: [String; 2],
    owner: String,
    rent_epoch: u64,
    lamports: u64,
    executable: bool,
}

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

#[derive(Serialize)]
pub struct SlotNotification {
    jsonrpc: &'static str,
    method: &'static str,
    params: SlotNotificationParams,
}

#[derive(Serialize)]
pub struct SlotNotificationParams {
    result: SlotNotificationResult,
    subscription: SubID,
}

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
