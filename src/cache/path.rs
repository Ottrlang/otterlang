use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::BaseDirs;

pub fn cache_root() -> Result<PathBuf> {
    if let Ok(custom) = env::var("OTTER_CACHE_DIR") {
        return Ok(PathBuf::from(custom));
    }

    let base_dirs = BaseDirs::new().context("failed to determine user directories")?;
    Ok(base_dirs.home_dir().join(".otter_cache"))
}

pub fn binaries_dir(root: &Path) -> PathBuf {
    root.join("binaries")
}

pub fn metadata_dir(root: &Path) -> PathBuf {
    root.join("metadata")
}

pub fn ensure_structure(root: &Path) -> Result<()> {
    let binaries = binaries_dir(root);
    let metadata = metadata_dir(root);
    std::fs::create_dir_all(binaries).context("failed to create binaries cache directory")?;
    std::fs::create_dir_all(metadata).context("failed to create metadata cache directory")?;
    Ok(())
}
