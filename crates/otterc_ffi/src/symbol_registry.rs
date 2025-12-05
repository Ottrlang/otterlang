use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use super::metadata::load_bridge_metadata;
use crate::types::BridgeMetadata;
use crate::types::FunctionSpec;

/// Represents a function that should be exported from a bridge crate.
#[derive(Clone, Debug)]
pub struct BridgeFunction {
    pub crate_name: String,
    pub spec: FunctionSpec,
}

#[derive(Clone, Default)]
pub struct BridgeSymbolRegistry {
    inner: Arc<Mutex<HashMap<String, BridgeMetadata>>>,
}

impl BridgeSymbolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Self {
        static GLOBAL: Lazy<BridgeSymbolRegistry> = Lazy::new(BridgeSymbolRegistry::new);
        &GLOBAL
    }

    pub fn record(&self, metadata: BridgeMetadata) {
        self.inner
            .lock()
            .insert(metadata.crate_name.clone(), metadata);
    }

    pub fn ensure_metadata(&self, crate_name: &str) -> Result<BridgeMetadata> {
        let mut guard = self.inner.lock();
        if let Some(existing) = guard.get(crate_name) {
            return Ok(existing.clone());
        }

        let metadata = load_bridge_metadata(crate_name)?;
        guard.insert(crate_name.to_string(), metadata.clone());
        Ok(metadata)
    }

    pub fn planned_functions(&self, crate_name: &str) -> Option<Vec<FunctionSpec>> {
        self.inner
            .lock()
            .get(crate_name)
            .map(|metadata| metadata.functions.clone())
    }
}
