pub mod benchmark;
pub mod config;
pub mod error;
pub mod ffi;
pub mod ffi_api;
pub mod introspection;
pub mod jit_stubs;
pub mod memory;
pub mod stdlib;
pub mod strings;
pub mod task;
pub mod version;

// Re-export jit_stubs as jit for compatibility
pub mod jit {
    pub use crate::jit_stubs::*;
}

// Re-export symbol types from otterc_symbols
pub mod symbol_registry {
    pub use otterc_symbols::{
        FfiFunction, FfiSignature, FfiType, GLOBAL_SYMBOL_REGISTRY, SymbolRegistry,
    };
}
