// Function Caching System
pub mod eviction;
pub mod function_cache;
pub mod metadata;

pub use eviction::{EvictionPolicy, LruEvictionPolicy};
pub use function_cache::FunctionCache;
pub use metadata::CacheMetadata;

use super::specialization::SpecializationKey;

/// Cache entry for a compiled function
#[derive(Debug, Clone)]
pub struct CachedFunction {
    pub key: SpecializationKey,
    pub code: Vec<u8>, // Compiled machine code
    pub metadata: CacheMetadata,
}

impl CachedFunction {
    pub fn new(key: SpecializationKey, code: Vec<u8>) -> Self {
        Self {
            key,
            code,
            metadata: CacheMetadata::default(),
        }
    }

    pub fn size(&self) -> usize {
        self.code.len()
    }
}
