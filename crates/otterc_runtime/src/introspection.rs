//! Runtime introspection module - Stub version
//!
//! This module provides runtime introspection capabilities. The full implementation
//! requires the otterc_jit crate. This stub provides minimal functionality.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::memory::GcStats;

/// Runtime introspection snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectionSnapshot {
    /// GC statistics
    pub gc_stats: Option<GcStats>,
    /// Timestamp
    pub timestamp_ms: u64,
}

impl IntrospectionSnapshot {
    pub fn capture() -> Self {
        let gc_stats = None;
        Self {
            gc_stats,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

/// Runtime introspection engine (stub)
pub struct IntrospectionEngine {
    snapshots: RwLock<Vec<IntrospectionSnapshot>>,
}

impl Default for IntrospectionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl IntrospectionEngine {
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(Vec::new()),
        }
    }

    pub fn capture_snapshot(&self) -> IntrospectionSnapshot {
        let snapshot = IntrospectionSnapshot::capture();
        self.snapshots.write().push(snapshot.clone());
        snapshot
    }

    pub fn get_snapshots(&self) -> Vec<IntrospectionSnapshot> {
        self.snapshots.read().clone()
    }

    pub fn clear_snapshots(&self) {
        self.snapshots.write().clear();
    }
}

/// Global introspection engine
static GLOBAL_ENGINE: once_cell::sync::Lazy<Arc<IntrospectionEngine>> =
    once_cell::sync::Lazy::new(|| Arc::new(IntrospectionEngine::new()));

/// Get the global introspection engine
pub fn get_introspection_engine() -> Arc<IntrospectionEngine> {
    Arc::clone(&GLOBAL_ENGINE)
}
