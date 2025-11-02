use parking_lot::RwLock;
use std::collections::HashMap;

use super::{CallSiteContext, RuntimeConstant, SpecializationKey};
use crate::ast::nodes::Function;

/// Manages function specialization
pub struct Specializer {
    specialized_versions: RwLock<HashMap<SpecializationKey, String>>,
    max_versions_per_function: usize,
}

impl Specializer {
    pub fn new() -> Self {
        Self {
            specialized_versions: RwLock::new(HashMap::new()),
            max_versions_per_function: 10,
        }
    }

    pub fn with_max_versions(max: usize) -> Self {
        Self {
            specialized_versions: RwLock::new(HashMap::new()),
            max_versions_per_function: max,
        }
    }

    /// Check if a specialization already exists
    pub fn has_specialization(&self, key: &SpecializationKey) -> bool {
        self.specialized_versions.read().contains_key(key)
    }

    /// Create a specialized version of a function
    pub fn specialize_function(
        &self,
        _function: &Function,
        context: &CallSiteContext,
    ) -> Result<SpecializationKey, String> {
        let key = context.specialization_key();

        // Check if we've already specialized this
        if self.has_specialization(&key) {
            return Ok(key);
        }

        // Check if we've exceeded the version limit
        let versions_count = self
            .specialized_versions
            .read()
            .keys()
            .filter(|k| k.function_name == key.function_name)
            .count();

        if versions_count >= self.max_versions_per_function {
            return Err(format!(
                "Max specialization versions ({}) exceeded for function {}",
                self.max_versions_per_function, key.function_name
            ));
        }

        // Create specialized function name
        let specialized_name = format!("{}_specialized_{}", key.function_name, key.to_string_key());

        // Store the specialization
        self.specialized_versions
            .write()
            .insert(key.clone(), specialized_name);

        Ok(key)
    }

    /// Get the specialized function name for a key
    pub fn get_specialized_name(&self, key: &SpecializationKey) -> Option<String> {
        self.specialized_versions.read().get(key).cloned()
    }

    /// Apply constant propagation optimizations based on known constants
    pub fn optimize_with_constants(
        &self,
        function: &Function,
        _constants: &[Option<RuntimeConstant>],
    ) -> Function {
        // For now, return the function as-is
        // In a full implementation, we would:
        // 1. Replace constant arguments with literals
        // 2. Simplify expressions using known constants
        // 3. Remove dead code paths
        function.clone()
    }

    /// Get statistics about specializations
    pub fn get_stats(&self) -> SpecializationStats {
        let versions = self.specialized_versions.read();
        let mut function_counts: HashMap<String, usize> = HashMap::new();

        for key in versions.keys() {
            *function_counts
                .entry(key.function_name.clone())
                .or_insert(0) += 1;
        }

        SpecializationStats {
            total_specializations: versions.len(),
            function_counts,
        }
    }
}

#[derive(Debug)]
pub struct SpecializationStats {
    pub total_specializations: usize,
    pub function_counts: HashMap<String, usize>,
}

impl Default for Specializer {
    fn default() -> Self {
        Self::new()
    }
}
