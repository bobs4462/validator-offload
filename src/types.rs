use std::collections::{hash_map::Drain, HashMap};

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

    pub fn remove(&mut self, key: &SubKey) -> Option<SubID> {
        let id = self.key2id.remove(key);
        if let Some(id) = id {
            self.id2key.remove(&id);
        }
        id
    }

    pub fn remove_rev(&mut self, id: &SubID) -> Option<SubKey> {
        let key = self.id2key.remove(&id);
        if let Some(ref key) = key {
            self.key2id.remove(key);
        }
        key
    }

    pub fn drain<'a>(&'a mut self) -> Drain<'a, SubKey, SubID> {
        self.key2id.drain()
    }

    pub fn drain_rev<'a>(&'a mut self) -> Drain<'a, SubID, SubKey> {
        self.id2key.drain()
    }

    pub fn get(&self, key: &SubKey) -> Option<&SubID> {
        self.key2id.get(key)
    }

    pub fn get_rev(&self, id: &SubID) -> Option<&SubKey> {
        self.id2key.get(id)
    }
}
