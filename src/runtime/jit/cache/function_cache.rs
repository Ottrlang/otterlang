use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use super::{CacheMetadata, CachedFunction, SpecializationKey};
use crate::runtime::jit::cache::eviction::{EvictionPolicy, LruEvictionPolicy};

/// Cache for compiled functions
pub struct FunctionCache {
    cache: Arc<RwLock<HashMap<SpecializationKey, CachedFunction>>>,
    eviction_policy: Arc<RwLock<LruEvictionPolicy>>,
    max_size_bytes: usize,
    current_size_bytes: Arc<RwLock<usize>>,
}

impl FunctionCache {
    pub fn new(max_size_bytes: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            eviction_policy: Arc::new(RwLock::new(LruEvictionPolicy::new())),
            max_size_bytes,
            current_size_bytes: Arc::new(RwLock::new(0)),
        }
    }

    /// Get a cached function
    pub fn get(&self, key: &SpecializationKey) -> Option<CachedFunction> {
        let cache = self.cache.write();
        if let Some(func) = cache.get(key) {
            self.eviction_policy.write().on_access(key);
            Some(func.clone())
        } else {
            None
        }
    }

    /// Store a compiled function
    pub fn put(
        &self,
        key: SpecializationKey,
        code: Vec<u8>,
        compilation_time: std::time::Duration,
    ) {
        let function_size = code.len();

        // Check if we need to evict
        let mut current_size = *self.current_size_bytes.read();
        while current_size + function_size > self.max_size_bytes {
            let evict_key = self.eviction_policy.write().evict(&self.cache.read());

            if let Some(key_to_evict) = evict_key {
                if let Some(func) = self.cache.write().remove(&key_to_evict) {
                    current_size -= func.size();
                    self.eviction_policy.write().on_remove(&key_to_evict);
                }
            } else {
                // Can't evict anything, but we'll still try to add
                break;
            }
        }

        // Add the new function
        let mut func = CachedFunction::new(key.clone(), code);
        func.metadata = CacheMetadata::new(compilation_time);

        let mut cache = self.cache.write();
        if let Some(old_func) = cache.insert(key.clone(), func) {
            current_size -= old_func.size();
        }
        current_size += function_size;

        *self.current_size_bytes.write() = current_size;
        self.eviction_policy.write().on_add(&key);
    }

    /// Check if a function is cached
    pub fn contains(&self, key: &SpecializationKey) -> bool {
        self.cache.read().contains_key(key)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read();
        CacheStats {
            total_functions: cache.len(),
            total_size_bytes: *self.current_size_bytes.read(),
            max_size_bytes: self.max_size_bytes,
            usage_percent: (*self.current_size_bytes.read() as f64 / self.max_size_bytes as f64)
                * 100.0,
        }
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.write().clear();
        *self.current_size_bytes.write() = 0;
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_functions: usize,
    pub total_size_bytes: usize,
    pub max_size_bytes: usize,
    pub usage_percent: f64,
}
