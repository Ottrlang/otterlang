//! OtterLang Rust FFI bridge modules.
//!
//! This module hosts the scaffolding for the cargo bridge pipeline that turns
//! `use rust:crate` imports into dynamically loaded shared libraries.

pub mod cargo_bridge;
pub mod dynamic_loader;
pub mod metadata;
pub mod rust_stubgen;
pub mod rustdoc_extractor;
pub mod symbol_registry;
pub mod types;

pub use cargo_bridge::{BridgeArtifacts, CargoBridge};
pub use dynamic_loader::{DynamicLibrary, DynamicLibraryLoader};
pub use metadata::load_bridge_functions;
pub use rust_stubgen::RustStubGenerator;
pub use rustdoc_extractor::{
    extract_crate_spec, extract_crate_spec_from_json, generate_rustdoc_json,
};
pub use symbol_registry::{BridgeFunction, BridgeSymbolRegistry};
pub use types::{
    BridgeMetadata, CallTemplate, CrateSpec, DependencyConfig, EnumVariant, EnumVariantKind, FnSig,
    FunctionSpec, PublicItem, RustPath, RustTypeRef, StructField, StubSource, TraitMethod,
    TypeSpec,
};
