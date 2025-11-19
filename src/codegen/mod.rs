pub mod llvm;
pub mod symbols;
pub mod target;

#[cfg(feature = "cranelift-backend")]
use tracing::warn;

pub use llvm::{
    BuildArtifact, CodegenOptLevel, CodegenOptions, build_executable, build_shared_library,
    current_llvm_version,
};
pub use symbols::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};
pub use target::TargetTriple;

// Re-export Cranelift backend when available
#[cfg(feature = "cranelift-backend")]
pub mod cranelift;
#[cfg(feature = "cranelift-backend")]
pub use cranelift::CraneliftBackend;

/// Codegen backend selection
#[derive(Debug, Clone, Copy)]
pub enum CodegenBackendType {
    LLVM,
    #[cfg(feature = "cranelift-backend")]
    Cranelift,
}

/// Build an executable using the backend specified in options
pub fn build_executable_with_backend(
    program: &ast::nodes::Program,
    expr_types: &std::collections::HashMap<usize, crate::typecheck::TypeInfo>,
    output: &std::path::Path,
    options: &CodegenOptions,
) -> anyhow::Result<BuildArtifact> {
    match options.backend {
        CodegenBackendType::LLVM => build_executable(program, expr_types, output, options),
        #[cfg(feature = "cranelift-backend")]
        CodegenBackendType::Cranelift => {
            match cranelift::build_executable(program, expr_types, output, options) {
                Ok(artifact) => Ok(artifact),
                Err(err) => {
                    warn!(
                        "Cranelift executable build failed ({}); falling back to LLVM",
                        err
                    );
                    build_executable(program, expr_types, output, options)
                }
            }
        }
    }
}

/// Build a shared library using the backend specified in options
pub fn build_shared_library_with_backend(
    program: &ast::nodes::Program,
    expr_types: &std::collections::HashMap<usize, crate::typecheck::TypeInfo>,
    output: &std::path::Path,
    options: &CodegenOptions,
) -> anyhow::Result<BuildArtifact> {
    match options.backend {
        CodegenBackendType::LLVM => build_shared_library(program, expr_types, output, options),
        #[cfg(feature = "cranelift-backend")]
        CodegenBackendType::Cranelift => {
            match cranelift::build_shared_library(program, expr_types, output, options) {
                Ok(artifact) => Ok(artifact),
                Err(err) => {
                    warn!(
                        "Cranelift shared library build failed ({}); falling back to LLVM",
                        err
                    );
                    build_shared_library(program, expr_types, output, options)
                }
            }
        }
    }
}
