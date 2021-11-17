use actix::Addr;
use clickhouse::{Client, Row};
use serde::Deserialize;
use std::sync::Arc;

use crate::{manager::SubscriptionManager, Commitment, Pubkey};

pub struct DatabaseListener {
    query: String,
    client: Client,
    managers: Arc<Vec<Addr<SubscriptionManager>>>,
}

impl DatabaseListener {
    pub fn new(
        query: String,
        client: Client,
        managers: Arc<Vec<Addr<SubscriptionManager>>>,
    ) -> Self {
        Self {
            query,
            client,
            managers,
        }
    }

    pub async fn listen(self) {
        let mut cursor = self
            .client
            .watch(&self.query)
            .fetch::<InsertNotification>()
            .unwrap();

        while let Ok(Some((_version, data))) = cursor.next().await {}
    }
}

#[derive(Deserialize, Row)]
pub struct InsertNotification {
    pubkey: Pubkey,
    slot_status: Commitment,
}
