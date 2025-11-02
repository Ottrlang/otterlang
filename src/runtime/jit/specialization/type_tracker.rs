use super::RuntimeType;
use std::collections::HashMap;

/// Tracks runtime types at call sites
pub struct TypeTracker {
    call_sites: HashMap<String, Vec<Vec<RuntimeType>>>,
}

impl TypeTracker {
    pub fn new() -> Self {
        Self {
            call_sites: HashMap::new(),
        }
    }

    /// Record a call site with its argument types
    pub fn record_call(&mut self, function_name: &str, arg_types: Vec<RuntimeType>) {
        self.call_sites
            .entry(function_name.to_string())
            .or_insert_with(Vec::new)
            .push(arg_types);
    }

    /// Get the most common type signature for a function
    pub fn get_common_signature(&self, function_name: &str) -> Option<Vec<RuntimeType>> {
        let sites = self.call_sites.get(function_name)?;
        if sites.is_empty() {
            return None;
        }

        // Simple heuristic: return the most frequent signature
        let mut counts: HashMap<&Vec<RuntimeType>, usize> = HashMap::new();
        for signature in sites {
            *counts.entry(signature).or_insert(0) += 1;
        }

        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(sig, _)| sig.clone())
    }

    /// Get all type signatures seen for a function
    pub fn get_signatures(&self, function_name: &str) -> Option<&Vec<Vec<RuntimeType>>> {
        self.call_sites.get(function_name)
    }

    /// Check if a function has multiple type signatures (polymorphic)
    pub fn is_polymorphic(&self, function_name: &str) -> bool {
        if let Some(sites) = self.call_sites.get(function_name) {
            if sites.len() < 2 {
                return false;
            }
            let first = &sites[0];
            sites.iter().any(|sig| sig != first)
        } else {
            false
        }
    }
}

impl Default for TypeTracker {
    fn default() -> Self {
        Self::new()
    }
}
