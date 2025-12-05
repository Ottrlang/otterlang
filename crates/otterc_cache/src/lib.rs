// Compilation cache management
pub mod manager;
pub mod metadata;
pub mod path;

// Re-exports for convenience
pub use manager::{CacheEntry, CacheManager};
pub use metadata::CacheMetadata;
pub use path::{cache_key_for_file, cache_root, ensure_cache_dir};

/// Build options for caching
#[derive(Debug, Clone)]
pub struct CacheBuildOptions {
    pub enable_cache: bool,
    pub cache_dir: std::path::PathBuf,
    pub max_cache_size: usize,
    pub release: bool,
    pub lto: bool,
    pub emit_ir: bool,
}

/// Compilation inputs for caching
#[derive(Debug, Clone)]
pub struct CompilationInputs {
    pub source_path: std::path::PathBuf,
    pub dependencies: Vec<String>,
    pub imports: Vec<String>,
}

impl CompilationInputs {
    pub fn new(source_path: std::path::PathBuf, dependencies: Vec<String>) -> Self {
        Self {
            source_path,
            dependencies,
            imports: Vec::new(),
        }
    }
}
