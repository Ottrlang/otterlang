use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub compiler_version: String,
    pub llvm_version: Option<String>,
    pub source: PathBuf,
    pub imports: Vec<PathBuf>,
    pub binary_path: PathBuf,
    pub binary_size: u64,
    pub build_time_ms: u128,
    pub options: CacheBuildOptions,
    pub linked_crates: Vec<String>,
}

impl CacheMetadata {
    pub fn new(
        key: String,
        compiler_version: impl Into<String>,
        llvm_version: Option<String>,
        source: PathBuf,
        imports: Vec<PathBuf>,
        binary_path: PathBuf,
        binary_size: u64,
        build_time_ms: u128,
        options: CacheBuildOptions,
        linked_crates: Vec<String>,
    ) -> Self {
        Self {
            key,
            created_at: Utc::now(),
            compiler_version: compiler_version.into(),
            llvm_version,
            source,
            imports,
            binary_path,
            binary_size,
            build_time_ms,
            options,
            linked_crates,
        }
    }

    pub fn file_stem(&self) -> String {
        self.key.clone()
    }

    pub fn write_to_yaml(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("failed to serialise cache metadata")?;
        let mut file = File::create(path).context("failed to open cache metadata file")?;
        file.write_all(yaml.as_bytes())
            .context("failed to write cache metadata")
    }

    pub fn read_from_yaml(path: &Path) -> Result<Self> {
        let file = File::open(path).context("failed to open cache metadata")?;
        Ok(serde_yaml::from_reader(file).context("failed to deserialise cache metadata")?)
    }

    pub fn binary_size(path: &Path) -> Result<u64> {
        let file = File::open(path)
            .with_context(|| format!("failed to open cached binary {}", path.display()))?;
        // SAFETY: The file descriptor stays alive for the duration of the mmap; we drop it immediately after measuring.
        let map = unsafe { Mmap::map(&file).context("failed to memory-map cached binary")? };
        Ok(map.len() as u64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheBuildOptions {
    pub release: bool,
    pub lto: bool,
    pub emit_ir: bool,
}

impl CacheBuildOptions {
    pub fn fingerprint(&self) -> String {
        format!(
            "release={}::lto={}::emit_ir={}",
            self.release, self.lto, self.emit_ir
        )
    }
}
