use std::collections::HashMap;

use crate::{SubID, SubKey};

/// Convenient type to act as a bidirectional map between
/// internal subscription identifier `SubKey` and client
/// assigned subscription identifier `SubID`. It's designed
/// in such way, that it ensures consistency between mappings
/// in both direction, so that one way mapping cannot exists
/// without reverse directional mapping.
#[derive(Default)]
pub struct SubscriptionsMap {
    key2id: HashMap<SubKey, SubID>,
    id2key: HashMap<SubID, SubKey>,
}

impl SubscriptionsMap {
    /// Insert a new entry, creating bidirectional mapping between SubKey and SubID
    pub fn insert(&mut self, key: SubKey, id: SubID) {
        self.key2id.insert(key.clone(), id);
        self.id2key.insert(id, key);
    }

    /// Remove entry by SubKey, also removes entry from reverse
    /// directional mapping from SubID to SubKey
    pub fn remove_by_key(&mut self, key: &SubKey) -> Option<SubID> {
        let id = self.key2id.remove(key);
        if let Some(id) = id {
            self.id2key.remove(&id);
        }
        id
    }

    /// Remove entry by SubID, also removes entry from SubKey to SubID mapping
    pub fn remove_by_id(&mut self, id: &SubID) -> Option<SubKey> {
        let key = self.id2key.remove(id);
        if let Some(ref key) = key {
            self.key2id.remove(key);
        }
        key
    }

    /// Clear map, return existing SubKey to SubID (usually for cleanup iteration)
    pub fn drain(&mut self) -> HashMap<SubKey, SubID> {
        self.id2key = HashMap::new();
        std::mem::take(&mut self.key2id)
    }

    /// Retrieve SubID that corresponds to specified SubKey, if such exists
    pub fn get_by_key(&self, key: &SubKey) -> Option<&SubID> {
        self.key2id.get(key)
    }

    /// Retrieve SubKey that corresponds to specified SubID, if such exists
    pub fn get_by_id(&self, id: &SubID) -> Option<&SubKey> {
        self.id2key.get(id)
    }
}
