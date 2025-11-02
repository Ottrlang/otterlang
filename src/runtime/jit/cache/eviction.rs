use super::{CachedFunction, SpecializationKey};
use std::collections::HashMap;

/// Eviction policy for function cache
pub trait EvictionPolicy: Send + Sync {
    /// Decide which function to evict
    fn evict(
        &mut self,
        cache: &HashMap<SpecializationKey, CachedFunction>,
    ) -> Option<SpecializationKey>;

    /// Called when a function is accessed
    fn on_access(&mut self, key: &SpecializationKey);

    /// Called when a function is added
    fn on_add(&mut self, key: &SpecializationKey);

    /// Called when a function is removed
    fn on_remove(&mut self, key: &SpecializationKey);
}

/// LRU (Least Recently Used) eviction policy
pub struct LruEvictionPolicy {
    access_order: Vec<SpecializationKey>,
}

impl LruEvictionPolicy {
    pub fn new() -> Self {
        Self {
            access_order: Vec::new(),
        }
    }
}

impl EvictionPolicy for LruEvictionPolicy {
    fn evict(
        &mut self,
        cache: &HashMap<SpecializationKey, CachedFunction>,
    ) -> Option<SpecializationKey> {
        // Return the least recently used key
        for key in &self.access_order {
            if cache.contains_key(key) {
                return Some(key.clone());
            }
        }
        None
    }

    fn on_access(&mut self, key: &SpecializationKey) {
        // Move to end (most recently used)
        self.access_order.retain(|k| k != key);
        self.access_order.push(key.clone());
    }

    fn on_add(&mut self, key: &SpecializationKey) {
        self.access_order.push(key.clone());
    }

    fn on_remove(&mut self, key: &SpecializationKey) {
        self.access_order.retain(|k| k != key);
    }
}

impl Default for LruEvictionPolicy {
    fn default() -> Self {
        Self::new()
    }
}
