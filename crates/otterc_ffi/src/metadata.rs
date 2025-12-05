use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use super::types::{BridgeMetadata, CallTemplate, DependencyConfig, FunctionSpec, TypeSpec};

#[derive(Clone, Debug, Deserialize)]
struct RawMetadata {
    #[serde(default)]
    dependency: Option<RawDependency>,
    #[serde(default)]
    functions: Vec<FunctionEntry>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawDependency {
    name: Option<String>,
    version: Option<String>,
    path: Option<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default = "default_true")]
    default_features: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct FunctionEntry {
    /// Canonical OtterLang export name (e.g. "reqwest:get").
    name: String,
    /// Optional symbol override. Defaults to a mangled variant of the export name.
    #[serde(default)]
    symbol: Option<String>,
    /// Fully-qualified Rust path for the function body (e.g. "reqwest::blocking::get").
    #[serde(default)]
    rust_path: Option<String>,
    /// Parameter type identifiers (Unit, Bool, I32, I64, F64, Str).
    #[serde(default)]
    params: Vec<String>,
    /// Return type identifier.
    result: String,
    /// Optional documentation string propagated into the generated stub.
    #[serde(default)]
    doc: Option<String>,
    #[serde(default)]
    call: CallConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
struct CallConfig {
    #[serde(default)]
    kind: CallKind,
    /// Optional expression template using placeholders {0}, {1}, ...
    expr: Option<String>,
}

impl Default for CallConfig {
    fn default() -> Self {
        Self {
            kind: CallKind::Direct,
            expr: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum CallKind {
    #[default]
    Direct,
    Result,
    Expr,
}

impl DependencyConfig {
    fn from_raw(crate_name: &str, base_dir: &Path, raw: Option<RawDependency>) -> Self {
        if let Some(raw) = raw {
            let path = raw.path.map(|p| resolve_dependency_path(base_dir, p));
            Self {
                name: raw.name.unwrap_or_else(|| crate_name.to_string()),
                version: raw.version,
                path,
                features: raw.features,
                default_features: raw.default_features,
            }
        } else {
            Self {
                name: crate_name.to_string(),
                version: None,
                path: None,
                features: Vec::new(),
                default_features: true,
            }
        }
    }
}

impl BridgeMetadata {
    fn from_raw(crate_name: &str, raw: RawMetadata) -> Result<Self> {
        let metadata_path = metadata_root().join(crate_name).join("bridge.yaml");
        let base_dir = metadata_path.parent().unwrap_or_else(|| Path::new("."));

        let dependency = DependencyConfig::from_raw(crate_name, base_dir, raw.dependency);
        let functions = raw
            .functions
            .into_iter()
            .map(|entry| entry.try_into_spec(&dependency))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            crate_name: crate_name.to_string(),
            dependency,
            functions,
        })
    }
}

/// Load the metadata for a given crate from `ffi/<crate>/bridge.yaml`. Absent files
/// resolve to an empty metadata structure to keep the bridge pipeline lenient.
pub fn load_bridge_metadata(crate_name: &str) -> Result<BridgeMetadata> {
    let metadata_path = metadata_root().join(crate_name).join("bridge.yaml");
    if !metadata_path.exists() {
        let base_dir = metadata_root();
        return Ok(BridgeMetadata {
            crate_name: crate_name.to_string(),
            dependency: DependencyConfig::from_raw(crate_name, base_dir.as_path(), None),
            functions: Vec::new(),
        });
    }

    let raw = fs::read_to_string(&metadata_path).with_context(|| {
        format!(
            "failed to read bridge metadata for crate `{crate_name}` at {}",
            metadata_path.display()
        )
    })?;

    let parsed: RawMetadata = serde_yaml::from_str(&raw).with_context(|| {
        format!(
            "failed to parse bridge metadata for crate `{crate_name}` at {}",
            metadata_path.display()
        )
    })?;

    BridgeMetadata::from_raw(crate_name, parsed)
}

impl FunctionEntry {
    fn try_into_spec(self, dependency: &DependencyConfig) -> Result<FunctionSpec> {
        let params = self
            .params
            .iter()
            .map(|ident| {
                parse_type(ident).with_context(|| type_error(&dependency.name, &self.name, ident))
            })
            .collect::<Result<Vec<_>>>()?;
        let result = parse_type(&self.result)
            .with_context(|| type_error(&dependency.name, &self.name, &self.result))?;

        let symbol = self
            .symbol
            .unwrap_or_else(|| default_symbol(&dependency.name, &self.name));

        let call = if let Some(expr) = self.call.expr.clone() {
            CallTemplate::Expr(expr)
        } else {
            match self.call.kind {
                CallKind::Direct => CallTemplate::Direct,
                CallKind::Result => CallTemplate::Result,
                CallKind::Expr => {
                    bail!(
                        "call.kind set to `expr` but no `expr` provided for {}:{}",
                        dependency.name,
                        self.name
                    )
                }
            }
        };

        Ok(FunctionSpec {
            name: self.name,
            symbol,
            params,
            result,
            doc: self.doc,
            rust_path: self.rust_path,
            call,
        })
    }
}

fn parse_type(identifier: &str) -> Result<TypeSpec> {
    match identifier.to_ascii_lowercase().as_str() {
        "unit" | "void" => Ok(TypeSpec::Unit),
        "bool" => Ok(TypeSpec::Bool),
        "i32" | "int32" => Ok(TypeSpec::I32),
        "i64" | "int64" => Ok(TypeSpec::I64),
        "f64" | "float64" | "double" => Ok(TypeSpec::F64),
        "str" | "string" => Ok(TypeSpec::Str),
        "opaque" | "handle" => Ok(TypeSpec::Opaque),
        other => Err(anyhow!(
            "unsupported FFI type identifier `{}` (expected unit, bool, i32, i64, f64, str, or opaque)",
            other
        )),
    }
}

fn default_symbol(crate_name: &str, export_name: &str) -> String {
    let mut base = export_name
        .chars()
        .map(|ch| match ch {
            ':' | '.' => '_',
            other => other,
        })
        .collect::<String>();
    if !base.starts_with(&format!("{crate_name}_")) {
        base = format!("{crate_name}_{base}");
    }
    format!("otter_{base}")
}

fn metadata_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ffi")
}

fn type_error(crate_name: &str, function: &str, ident: &str) -> String {
    format!("unsupported FFI type identifier `{ident}` in {crate_name}:{function}")
}

const fn default_true() -> bool {
    true
}

fn resolve_dependency_path(base_dir: &Path, path: String) -> PathBuf {
    let candidate = PathBuf::from(&path);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        base_dir.join(candidate)
    };

    joined.canonicalize().unwrap_or(joined)
}

/// Convenience helper retained for older call sites that only require the
/// function list.
pub fn load_bridge_functions(crate_name: &str) -> Result<Vec<FunctionSpec>> {
    Ok(load_bridge_metadata(crate_name)?.functions)
}
