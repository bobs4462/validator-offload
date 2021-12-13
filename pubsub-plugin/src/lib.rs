use std::sync::Arc;

use bytes::Bytes;
use rmp_serde as rmps;
use serde::Serialize;
use solana_accountsdb_plugin_interface::accountsdb_plugin_interface::ReplicaAccountInfoVersions;
use solana_accountsdb_plugin_interface::accountsdb_plugin_interface::SlotStatus as AccDBSlotStatus;
use tokio_nsq::NSQTopic;

enum Payload {
    Account(AccountData),
    Slot(SlotData),
}

struct Message {
    payload: Payload,
    topic: Arc<NSQTopic>,
}

impl Message {
    fn serialize(&self) -> Option<Vec<u8>> {
        match &self.payload {
            Payload::Slot(v) => rmps::to_vec(v).ok(),
            Payload::Account(v) => rmps::to_vec(v).ok(),
        }
    }

    fn from_account(
        account: ReplicaAccountInfoVersions<'_>,
        slot: Slot,
        topic: Arc<NSQTopic>,
    ) -> Self {
        let mut account = AccountData::from(account);
        account.slot = slot;
        let payload = Payload::Account(account);
        Self { payload, topic }
    }

    fn from_slot(slot: Slot, parent: Slot, status: AccDBSlotStatus, topic: Arc<NSQTopic>) -> Self {
        let slot = SlotData {
            slot,
            parent,
            status: status.into(),
        };
        let payload = Payload::Slot(slot);
        Self { payload, topic }
    }
}

type Pubkey = [u8; 32];

type Slot = u64;

#[derive(Serialize)]
struct AccountData {
    /// Public key of given account
    pubkey: Pubkey,
    /// Public key owner program (if any) of given account
    owner: Pubkey,
    lamports: u64,
    data: Bytes,
    rent_epoch: u64,
    executable: bool,
    /// Slot number at which the update was generated
    slot: Slot,
}

#[derive(Serialize)]
struct SlotData {
    slot: Slot,
    parent: Slot,
    status: Commitment,
}

#[derive(Serialize)]
enum Commitment {
    Processed = 1,
    Confirmed = 2,
    Finalized = 3,
}

impl From<AccDBSlotStatus> for Commitment {
    fn from(status: AccDBSlotStatus) -> Self {
        match status {
            AccDBSlotStatus::Processed => Self::Processed,
            AccDBSlotStatus::Confirmed => Self::Confirmed,
            AccDBSlotStatus::Rooted => Self::Finalized,
        }
    }
}

impl From<ReplicaAccountInfoVersions<'_>> for AccountData {
    fn from(src: ReplicaAccountInfoVersions<'_>) -> Self {
        match src {
            ReplicaAccountInfoVersions::V0_0_1(acc) => {
                let mut pubkey = [0; 32];
                pubkey.copy_from_slice(acc.pubkey);
                let mut owner = [0; 32];
                owner.copy_from_slice(acc.owner);
                let data = Bytes::copy_from_slice(acc.data);
                Self {
                    pubkey,
                    owner,
                    lamports: acc.lamports,
                    data,
                    rent_epoch: acc.rent_epoch,
                    executable: acc.executable,
                    slot: 0,
                }
            }
        }
    }
}

mod plugin;
mod publisher;
