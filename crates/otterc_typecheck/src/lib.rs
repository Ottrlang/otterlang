//! `OtterLang` type checking utilities shared across the compiler components.
#![allow(
    clippy::missing_docs_in_private_items,
    reason = "The legacy typechecker still lacks detailed docs; tracked separately."
)]

pub mod checker;
pub mod diagnostics;
pub mod types;

pub use checker::TypeChecker;
pub use diagnostics::from_type_errors as diagnostics_from_type_errors;
pub use types::{EnumLayout, TypeContext, TypeError, TypeInfo};
