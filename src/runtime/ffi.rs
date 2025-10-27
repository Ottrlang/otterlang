use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use abi_stable::std_types::{RString, RVec};
use abi_stable::StableAbi;
use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use libloading::Library;
use once_cell::sync::OnceCell;
use sha1::{Digest, Sha1};
use tracing::debug;

use crate::cache::path::cache_root;
use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

#[derive(Clone, Copy)]
pub struct SymbolProvider {
    pub register: fn(&SymbolRegistry),
}

inventory::collect!(crate::runtime::ffi::SymbolProvider);

pub struct Runtime {
    registry: &'static SymbolRegistry,
    libraries: HashMap<String, Library>,
    ffi_root: PathBuf,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let registry = bootstrap_stdlib();

        let ffi_root = cache_root()?.join("ffi");
        fs::create_dir_all(&ffi_root).context("failed to create ffi cache directory")?;

        Ok(Self {
            registry,
            libraries: HashMap::new(),
            ffi_root,
        })
    }

    pub fn symbols(&self) -> &SymbolRegistry {
        &self.registry
    }

    pub fn load_crate(&mut self, crate_name: &str) -> Result<()> {
        if self.libraries.contains_key(crate_name) {
            return Ok(());
        }

        let (metadata, package) = resolve_package(crate_name)?;

        let fingerprint = fingerprint_package(&package)?;
        let lib_filename = libloading::library_filename(&package.name);

        let cache_dir = self.ffi_root.join(&package.name).join(&fingerprint);
        fs::create_dir_all(&cache_dir)
            .with_context(|| format!("failed to create ffi cache path {}", cache_dir.display()))?;
        let cached_lib = cache_dir.join(&lib_filename);

        if !cached_lib.exists() {
            build_crate(&metadata, &package)?;
            let built_path = metadata
                .target_directory
                .as_std_path()
                .join("release")
                .join(&lib_filename);
            fs::copy(&built_path, &cached_lib).with_context(|| {
                format!(
                    "failed to cache built library {} to {}",
                    built_path.display(),
                    cached_lib.display()
                )
            })?;
        }

        let library = unsafe { Library::new(&cached_lib) }
            .with_context(|| format!("failed to load ffi library {}", cached_lib.display()))?;

        if let Err(err) = register_dynamic_exports(&library, self.registry) {
            debug!(error = %err, "ffi register failed");
        }

        self.libraries.insert(crate_name.to_string(), library);

        Ok(())
    }
}

fn register_builtin_symbols(registry: &SymbolRegistry) {
    for provider in inventory::iter::<SymbolProvider> {
        (provider.register)(registry);
    }
}

fn build_crate(metadata: &Metadata, package: &Package) -> Result<()> {
    let manifest_path = package.manifest_path.as_std_path();
    let status = duct::cmd!(
        "cargo",
        "build",
        "--release",
        "--manifest-path",
        manifest_path
    )
    .dir(
        manifest_path
            .parent()
            .unwrap_or_else(|| metadata.workspace_root.as_std_path()),
    )
    .run()
    .context("failed to build ffi crate")?;

    if !status.status.success() {
        bail!("cargo build failed for crate {}", package.name);
    }

    Ok(())
}

fn fingerprint_package(package: &Package) -> Result<String> {
    let manifest = fs::read(&package.manifest_path).with_context(|| {
        format!(
            "failed to read manifest {}",
            package.manifest_path.as_std_path().display()
        )
    })?;
    let mut hasher = Sha1::new();
    hasher.update(manifest);
    hasher.update(package.version.to_string().as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

#[repr(C)]
#[derive(Clone, StableAbi)]
pub struct StableFunction {
    pub name: RString,
    pub symbol: RString,
    pub params: RVec<FfiType>,
    pub result: FfiType,
}

#[repr(C)]
#[derive(Clone, StableAbi)]
pub struct StableExportSet {
    pub functions: RVec<StableFunction>,
}

type ExportFn = extern "C" fn() -> StableExportSet;

fn register_dynamic_exports(library: &Library, registry: &SymbolRegistry) -> Result<()> {
    unsafe {
        let exports = library
            .get::<ExportFn>(b"otterlang_exports")
            .context("ffi module missing otterlang_exports symbol")?;
        let set = exports();
        for function in set.functions.into_iter() {
            registry.register(FfiFunction {
                name: function.name.into_string(),
                symbol: function.symbol.into_string(),
                signature: FfiSignature::new(function.params.into_vec(), function.result),
            });
        }
    }

    Ok(())
}

fn resolve_package(crate_name: &str) -> Result<(Metadata, Package)> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("failed to execute cargo metadata")?;

    if let Some(package) = metadata
        .packages
        .iter()
        .find(|pkg| pkg.name == crate_name)
        .cloned()
    {
        return Ok((metadata, package));
    }

    let workspace_root = metadata.workspace_root.as_std_path();
    let alt_manifest = workspace_root
        .join("ffi")
        .join(crate_name)
        .join("Cargo.toml");

    if alt_manifest.exists() {
        let alt_metadata = MetadataCommand::new()
            .manifest_path(&alt_manifest)
            .exec()
            .with_context(|| {
                format!("failed to execute cargo metadata for crate `{crate_name}`")
            })?;
        let package = alt_metadata
            .packages
            .iter()
            .find(|pkg| pkg.name == crate_name)
            .cloned()
            .ok_or_else(|| anyhow!("crate `{crate_name}` not found in metadata"))?;
        return Ok((alt_metadata, package));
    }

    Err(anyhow!("crate `{crate_name}` not found"))
}

static STD_INIT: OnceCell<()> = OnceCell::new();

pub fn bootstrap_stdlib() -> &'static SymbolRegistry {
    let registry = SymbolRegistry::global();
    STD_INIT.get_or_init(|| {
        register_builtin_symbols(registry);
    });
    registry
}
