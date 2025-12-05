use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use libloading::Library;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Thin wrapper around a `libloading::Library` with reference counting so callers
/// can clone handles when sharing across the runtime.
#[derive(Clone)]
pub struct DynamicLibrary {
    inner: Arc<Library>,
}

impl DynamicLibrary {
    pub fn new(library: Library) -> Self {
        Self {
            inner: Arc::new(library),
        }
    }
}

impl Deref for DynamicLibrary {
    type Target = Library;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

/// Loads bridge crates on demand and caches the handles.
pub struct DynamicLibraryLoader {
    cache: Mutex<HashMap<PathBuf, DynamicLibrary>>,
}

impl Default for DynamicLibraryLoader {
    fn default() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl DynamicLibraryLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Self {
        static GLOBAL: Lazy<DynamicLibraryLoader> = Lazy::new(DynamicLibraryLoader::new);
        &GLOBAL
    }

    pub fn load(&self, path: &Path) -> Result<DynamicLibrary> {
        if let Some(existing) = self.cache.lock().get(path).cloned() {
            return Ok(existing);
        }

        let library = unsafe { Library::new(path) }
            .with_context(|| format!("failed to load dynamic library {}", path.display()))?;
        let handle = DynamicLibrary::new(library);
        self.cache.lock().insert(path.to_path_buf(), handle.clone());
        Ok(handle)
    }
}
