// JIT Runtime System Module
pub mod adaptive;
pub mod cache;
pub mod concurrency;
pub mod engine;
pub mod executor;
pub mod layout;
pub mod optimization;
pub mod profiler;
pub mod specialization;

pub use concurrency::ConcurrencyManager;
pub use engine::JitEngine;
pub use executor::JitExecutor;
pub use layout::DataLayoutOptimizer;
