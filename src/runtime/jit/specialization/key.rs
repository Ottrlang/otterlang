use ahash::AHasher;
use std::hash::{Hash, Hasher};

use super::{RuntimeConstant, RuntimeType};

/// Unique key identifying a specialized function version
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecializationKey {
    pub function_name: String,
    pub arg_types: Vec<RuntimeType>,
    pub arg_constants_hash: u64,
}

impl SpecializationKey {
    pub fn new(
        function_name: String,
        arg_types: Vec<RuntimeType>,
        arg_constants: Vec<Option<RuntimeConstant>>,
    ) -> Self {
        let mut hasher = AHasher::default();
        for constant in &arg_constants {
            if let Some(c) = constant {
                // Hash the constant value
                match c {
                    RuntimeConstant::Bool(b) => b.hash(&mut hasher),
                    RuntimeConstant::I32(i) => i.hash(&mut hasher),
                    RuntimeConstant::I64(i) => i.hash(&mut hasher),
                    RuntimeConstant::F64(f) => f.to_bits().hash(&mut hasher),
                    RuntimeConstant::Str(s) => s.hash(&mut hasher),
                }
            } else {
                None::<u64>.hash(&mut hasher);
            }
        }
        let arg_constants_hash = hasher.finish();

        Self {
            function_name,
            arg_types,
            arg_constants_hash,
        }
    }

    pub fn to_string_key(&self) -> String {
        let types_str = self
            .arg_types
            .iter()
            .map(|t| format!("{:?}", t))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}_<{}>_{:x}",
            self.function_name, types_str, self.arg_constants_hash
        )
    }
}
