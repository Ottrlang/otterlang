use std::time::{Duration, Instant};

/// Metadata for cached functions
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    pub created_at: Instant,
    pub last_accessed: Instant,
    pub access_count: u64,
    pub compilation_time: Duration,
}

impl CacheMetadata {
    pub fn new(compilation_time: Duration) -> Self {
        let now = Instant::now();
        Self {
            created_at: now,
            last_accessed: now,
            access_count: 1,
            compilation_time,
        }
    }

    pub fn record_access(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }
}

impl Default for CacheMetadata {
    fn default() -> Self {
        Self::new(Duration::ZERO)
    }
}
