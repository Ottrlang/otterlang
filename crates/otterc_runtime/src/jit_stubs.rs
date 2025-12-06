//! Stub types for JIT functionality that will be provided by otterc_jit crate.
//! These stubs allow the runtime to compile without the JIT dependency.

use serde::{Deserialize, Serialize};

/// Tiered compilation configuration (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredConfig {
    /// Enable tiered compilation
    pub enabled: bool,
    /// Hot threshold for tier-up
    pub hot_threshold: u64,
    /// Very hot threshold for aggressive optimization
    pub very_hot_threshold: u64,
}

impl Default for TieredConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hot_threshold: 1000,
            very_hot_threshold: 10000,
        }
    }
}

impl TieredConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(val) = std::env::var("OTTER_TIER_ENABLED") {
            config.enabled = val.parse().unwrap_or(true);
        }
        if let Ok(val) = std::env::var("OTTER_HOT_THRESHOLD") {
            config.hot_threshold = val.parse().unwrap_or(1000);
        }
        config
    }
}

/// Tiered compilation statistics (stub)
#[derive(Debug, Clone, Default)]
pub struct TieredStats {
    pub functions_compiled: usize,
    pub tier1_count: usize,
    pub tier2_count: usize,
}

/// Compilation tier (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTier {
    Interpreter,
    Tier1,
    Tier2,
}

/// Tiered compiler (stub)
pub struct TieredCompiler {
    _config: TieredConfig,
}

impl TieredCompiler {
    pub fn new(_config: TieredConfig) -> Self {
        Self {
            _config: TieredConfig::default(),
        }
    }

    pub fn get_stats(&self) -> TieredStats {
        TieredStats::default()
    }
}

/// Compilation profiler (stub)
pub struct CompilationProfiler;
impl CompilationProfiler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CompilationProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Function metrics (stub)
#[derive(Debug, Clone, Default)]
pub struct FunctionMetrics {
    pub call_count: u64,
    pub total_time_ns: u64,
}

/// Memory profiler (stub)
pub struct MemoryProfiler;
impl MemoryProfiler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

pub mod tiered_compiler {
    pub use super::{CompilationTier, TieredCompiler, TieredConfig, TieredStats};
}

pub mod profiler {
    pub use super::{CompilationProfiler, FunctionMetrics, MemoryProfiler};
}
