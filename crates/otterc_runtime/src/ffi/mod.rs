mod dynamic;
mod exports;
mod providers;

use crate::symbol_registry::SymbolRegistry;
use anyhow::Result;

pub use dynamic::DynamicLibraryBackend;
pub use exports::{ExportFn, StableExportSet, StableFunction, register_dynamic_exports};
pub use providers::{SymbolProvider, bootstrap_stdlib};

pub trait FfiBackend {
    fn symbols(&self) -> &SymbolRegistry;
    fn load_crate(&mut self, crate_name: &str) -> Result<()>;
    fn call_json(&mut self, crate_name: &str, func: &str, args_json: &str) -> Result<String>;
}

/// Returns a boxed backend that uses dynamic libraries for FFI.
pub fn new_backend() -> Result<Box<dyn FfiBackend>> {
    Ok(Box::new(DynamicLibraryBackend::new()?))
}
