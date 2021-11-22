use std::collections::HashMap;

use crate::{SubID, SubKey};

#[derive(Default)]
pub struct SubscriptionsMap {
    key2id: HashMap<SubKey, SubID>,
    id2key: HashMap<SubID, SubKey>,
}

impl SubscriptionsMap {
    pub fn insert(&mut self, key: SubKey, id: SubID) {
        self.key2id.insert(key.clone(), id);
        self.id2key.insert(id, key);
    }

    pub fn remove_by_key(&mut self, key: &SubKey) -> Option<SubID> {
        let id = self.key2id.remove(key);
        if let Some(id) = id {
            self.id2key.remove(&id);
        }
        id
    }

    pub fn remove_by_id(&mut self, id: &SubID) -> Option<SubKey> {
        let key = self.id2key.remove(&id);
        if let Some(ref key) = key {
            self.key2id.remove(key);
        }
        key
    }

    pub fn drain(&mut self) -> HashMap<SubKey, SubID> {
        self.id2key = HashMap::new();
        std::mem::replace(&mut self.key2id, HashMap::new())
    }

    pub fn get_by_key(&self, key: &SubKey) -> Option<&SubID> {
        self.key2id.get(key)
    }

    pub fn get_by_id(&self, id: &SubID) -> Option<&SubKey> {
        self.id2key.get(id)
    }
}
