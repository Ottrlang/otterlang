use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use otterlang::cache::{CacheBuildOptions, CacheManager, CacheMetadata, CompilationInputs};

fn temp_cache_dir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("otter-cache-test")
        .tempdir()
        .expect("failed to create temp dir")
}

#[test]
fn cache_roundtrip_stores_and_loads_metadata() -> Result<()> {
    let cache_dir = temp_cache_dir();
    std::env::set_var("OTTER_CACHE_DIR", cache_dir.path());

    let manager = CacheManager::new()?;

    let source_path = cache_dir.path().join("hello.otter");
    fs::write(&source_path, "print(\"hi\")\n")?;

    let inputs = CompilationInputs::new(source_path.clone(), Vec::new());
    let options = CacheBuildOptions {
        release: false,
        lto: false,
        emit_ir: false,
    };

    let key = manager.fingerprint(&inputs, &options, "test-version")?;
    let binary_path = manager.binary_path(&key);
    fs::write(&binary_path, b"fake-binary")?;

    let binary_size = CacheMetadata::binary_size(&binary_path)?;
    let metadata = CacheMetadata::new(
        key.as_str().to_string(),
        "test-version",
        Some("llvm-test".to_string()),
        canonical_or(&source_path),
        Vec::new(),
        binary_path.clone(),
        binary_size,
        1,
        options.clone(),
        Vec::new(),
    );

    manager.store(&metadata)?;

    let entry = manager
        .lookup(&key)?
        .expect("expected cache entry to roundtrip");

    assert_eq!(entry.metadata.binary_size, binary_size);
    assert_eq!(entry.metadata.compiler_version, "test-version");
    assert_eq!(entry.binary_path, binary_path);

    std::env::remove_var("OTTER_CACHE_DIR");
    Ok(())
}

#[test]
fn cache_key_changes_with_content() -> Result<()> {
    let cache_dir = temp_cache_dir();
    std::env::set_var("OTTER_CACHE_DIR", cache_dir.path());

    let manager = CacheManager::new()?;

    let source_path = cache_dir.path().join("mutate.otter");
    fs::write(&source_path, "print(\"first\")\n")?;
    let inputs = CompilationInputs::new(source_path.clone(), Vec::new());
    let options = CacheBuildOptions {
        release: false,
        lto: false,
        emit_ir: false,
    };

    let first_key = manager.fingerprint(&inputs, &options, "test-version")?;

    fs::write(&source_path, "print(\"second\")\n")?;
    let second_key = manager.fingerprint(&inputs, &options, "test-version")?;

    assert_ne!(first_key.as_str(), second_key.as_str());

    std::env::remove_var("OTTER_CACHE_DIR");
    Ok(())
}

fn canonical_or(path: &PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.clone())
}
