use serde::Serialize;

use crate::{
    message::{AccountInfo, AccountUpdatedMessage},
    Slot, SubID, SubscriptionKind,
};

#[derive(Serialize)]
pub struct Notification {
    jsonrpc: &'static str,
    method: &'static str,
    params: NotificationParams,
}

#[derive(Serialize)]
struct NotificationParams {
    result: NotificationResult,
    subscription: SubID,
}

#[derive(Serialize)]
struct NotificationResult {
    context: NotificationContext,
    value: NotificationValue,
}

#[derive(Serialize)]
struct NotificationContext {
    slot: Slot,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum NotificationValue {
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

impl Notification {
    const JSONRPC: &'static str = "2.0";
    pub fn new(msg: AccountUpdatedMessage, subscription: SubID) -> Self {
        let method = match msg.key.kind {
            SubscriptionKind::Program => "programNotification",
            SubscriptionKind::Acccount => "accountNotification",
        };
        let result = NotificationResult::from(msg);
        let params = NotificationParams {
            result,
            subscription,
        };

        Self {
            jsonrpc: Self::JSONRPC,
            method,
            params,
        }
    }
}

impl From<AccountUpdatedMessage> for NotificationResult {
    fn from(msg: AccountUpdatedMessage) -> Self {
        let context = NotificationContext {
            slot: msg.info.slot,
        };

        let value = NotificationValue::from(msg);

        Self { context, value }
    }
}

impl From<AccountUpdatedMessage> for NotificationValue {
    fn from(msg: AccountUpdatedMessage) -> Self {
        let account = AccountValue::from(msg.info);

        match msg.key.kind {
            SubscriptionKind::Program => {
                let pubkey = bs58::encode(msg.key.key).into_string();
                let value = ProgramValue { pubkey, account };
                Self::Program(value)
            }
            SubscriptionKind::Acccount => Self::Account(account),
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
            zstd::encode_all(data.as_slice(), 0).expect("Account data cannot be compressed"),
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
