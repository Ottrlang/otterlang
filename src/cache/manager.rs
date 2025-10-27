use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use sha1::{Digest, Sha1};
use tracing::debug;

use crate::cache::metadata::{CacheBuildOptions, CacheMetadata};
use crate::cache::path::{binaries_dir, cache_root, ensure_structure, metadata_dir};

#[derive(Clone, Debug)]
pub struct CacheKey(pub Arc<String>);

impl CacheKey {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug)]
pub struct CacheEntry {
    pub key: CacheKey,
    pub metadata: CacheMetadata,
    pub binary_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CompilationInputs {
    pub primary: PathBuf,
    pub imports: Vec<PathBuf>,
}

impl CompilationInputs {
    pub fn new(primary: PathBuf, imports: Vec<PathBuf>) -> Self {
        Self { primary, imports }
    }

    pub fn all_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::with_capacity(1 + self.imports.len());
        files.push(self.primary.clone());
        files.extend(self.imports.iter().cloned());
        files
    }
}

pub struct CacheManager {
    root: PathBuf,
    binaries_dir: PathBuf,
    metadata_dir: PathBuf,
}

impl CacheManager {
    pub fn new() -> Result<Self> {
        let root = cache_root()?;
        ensure_structure(&root)?;
        let binaries = binaries_dir(&root);
        let metadata = metadata_dir(&root);

        debug!("cache root initialised" = %root.display());

        Ok(Self {
            root,
            binaries_dir: binaries,
            metadata_dir: metadata,
        })
    }

    pub fn cache_root(&self) -> &Path {
        &self.root
    }

    pub fn binary_path(&self, key: &CacheKey) -> PathBuf {
        let suffix = env_suffix();
        self.binaries_dir
            .join(format!("{}{}", key.as_str(), suffix))
    }

    pub fn metadata_path(&self, key: &CacheKey) -> PathBuf {
        self.metadata_dir.join(format!("{}.yaml", key.as_str()))
    }

    pub fn fingerprint(
        &self,
        inputs: &CompilationInputs,
        options: &CacheBuildOptions,
        compiler_version: &str,
    ) -> Result<CacheKey> {
        let mut files = inputs
            .all_files()
            .into_iter()
            .map(|path| canonicalise(path))
            .collect::<Result<Vec<_>>>()?;

        files.sort();
        files.dedup();

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::with_template("hashing [{elapsed_precise}] {wide_bar} {pos}/{len}")
                .unwrap()
                .progress_chars("=> "),
        );

        let file_hashes = files
            .par_iter()
            .map(|path| {
                let pb = pb.clone();
                hash_file(path, pb)
            })
            .collect::<Result<Vec<_>>>()?;

        pb.finish_and_clear();

        let mut hasher = Sha1::new();
        for (path, digest) in file_hashes {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(&digest);
        }

        hasher.update(options.fingerprint().as_bytes());
        hasher.update(compiler_version.as_bytes());

        let key = format!("{:x}", hasher.finalize());
        Ok(CacheKey(Arc::new(key)))
    }

    pub fn lookup(&self, key: &CacheKey) -> Result<Option<CacheEntry>> {
        let metadata_path = self.metadata_path(key);
        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata = CacheMetadata::read_from_yaml(&metadata_path)?;
        let binary_path = metadata.binary_path.clone();

        if !binary_path.exists() {
            debug!("cached binary missing" = %binary_path.display());
            return Ok(None);
        }

        Ok(Some(CacheEntry {
            key: key.clone(),
            metadata,
            binary_path,
        }))
    }

    pub fn store(&self, metadata: &CacheMetadata) -> Result<()> {
        let metadata_path = self.metadata_path(&CacheKey(Arc::new(metadata.key.clone())));
        metadata.write_to_yaml(&metadata_path)
    }
}

fn canonicalise(path: PathBuf) -> Result<PathBuf> {
    fs::canonicalize(&path)
        .map_err(|err| anyhow!("failed to canonicalize {}: {err}", path.display()))
}

fn hash_file(path: &Path, pb: ProgressBar) -> Result<(PathBuf, Vec<u8>)> {
    let data =
        fs::read(path).with_context(|| format!("failed to read {} for hashing", path.display()))?;
    let digest = Sha1::digest(&data);
    pb.inc(1);
    Ok((path.to_path_buf(), digest.to_vec()))
}

fn env_suffix() -> &'static str {
    if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    }
}
