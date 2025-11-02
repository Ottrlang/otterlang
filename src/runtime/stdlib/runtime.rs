use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "task-runtime")]
use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use sysinfo::System;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};
use crate::version::VERSION;

#[cfg(feature = "task-runtime")]
use crate::runtime::task::{TaskMetricsSnapshot, TaskRuntimeMetrics};

// ============================================================================
// Runtime Statistics Tracking
// ============================================================================

static ACTIVE_GOROUTINES: AtomicU64 = AtomicU64::new(0);

// Track runtime statistics
#[derive(Default)]
struct RuntimeStats {
    #[allow(dead_code)]
    total_tasks: u64,
    #[allow(dead_code)]
    completed_tasks: u64,
    #[allow(dead_code)]
    active_threads: usize,
    heap_bytes: usize,
    cpu_count: usize,
}

static RUNTIME_STATS: Lazy<RwLock<RuntimeStats>> =
    Lazy::new(|| RwLock::new(RuntimeStats::default()));

#[cfg(feature = "task-runtime")]
static TASK_METRICS: Lazy<RwLock<Option<Arc<TaskRuntimeMetrics>>>> =
    Lazy::new(|| RwLock::new(None));

// ============================================================================
// Runtime Functions
// ============================================================================

/// Get the number of active tasks/threads
#[no_mangle]
pub extern "C" fn otter_runtime_gos() -> i64 {
    ACTIVE_GOROUTINES.load(Ordering::SeqCst) as i64
}

/// Get the number of CPU cores
#[no_mangle]
pub extern "C" fn otter_runtime_cpu_count() -> i64 {
    let mut system = System::new_all();
    system.refresh_cpu();
    let count = system.cpus().len();

    // Update stats
    RUNTIME_STATS.write().cpu_count = count;

    count as i64
}

/// Get current heap memory usage in bytes
/// Note: In Rust, we don't have direct heap access, so we approximate
/// using process memory from sysinfo
#[no_mangle]
pub extern "C" fn otter_runtime_memory() -> i64 {
    let mut system = System::new_all();
    system.refresh_memory();

    // Get current process memory (approximation of heap)
    let process_id = std::process::id();
    if let Some(process) = system.process(sysinfo::Pid::from(process_id as usize)) {
        let memory_bytes = process.memory() * 1024; // sysinfo returns KB
        RUNTIME_STATS.write().heap_bytes = memory_bytes as usize;
        memory_bytes as i64
    } else {
        // Fallback: use system memory info
        let used_memory = system.used_memory() * 1024;
        RUNTIME_STATS.write().heap_bytes = used_memory as usize;
        used_memory as i64
    }
}

/// Trigger garbage collection
/// Note: Rust doesn't have GC, but we can:
/// 1. Drop unused allocations
/// 2. Trigger memory compaction if available
/// 3. Clear caches
#[no_mangle]
pub extern "C" fn otter_runtime_collect_garbage() {
    // In Rust, we don't have explicit GC, but we can:
    // 1. Suggest memory cleanup to the allocator
    // 2. Clear any runtime caches

    let _ = Vec::<u8>::with_capacity(1024);
}

/// Get runtime statistics as a JSON string
/// Returns a JSON object with various runtime metrics
#[no_mangle]
pub extern "C" fn otter_runtime_stats() -> *mut c_char {
    let stats = RUNTIME_STATS.read();

    // Build JSON string manually (avoiding serde_json dependency for simplicity)
    let mut system = System::new_all();
    system.refresh_cpu();
    system.refresh_memory();

    let cpu_count = system.cpus().len();
    let active_gos = ACTIVE_GOROUTINES.load(Ordering::SeqCst);
    let memory_bytes = stats.heap_bytes;

    #[cfg_attr(not(feature = "task-runtime"), allow(unused_mut))]
    let mut fields = vec![
        format!("\"gos\":{}", active_gos),
        format!("\"cpu_count\":{}", cpu_count),
        format!("\"memory_bytes\":{}", memory_bytes),
        format!("\"total_memory\":{}", system.total_memory() * 1024),
        format!("\"available_memory\":{}", system.available_memory() * 1024),
    ];

    #[cfg(feature = "task-runtime")]
    if let Some(task_field) = task_metrics_field() {
        fields.push(task_field);
    }

    let json = format!("{{{}}}", fields.join(","));

    CString::new(json)
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Get OtterLang runtime version
#[no_mangle]
pub extern "C" fn otter_runtime_version() -> *mut c_char {
    CString::new(VERSION)
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Free a string returned by runtime functions
#[no_mangle]
pub extern "C" fn otter_runtime_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

// ============================================================================
// Helper Functions for Sync Module Integration
// ============================================================================

/// Increment active task count
pub fn increment_active_tasks() {
    ACTIVE_GOROUTINES.fetch_add(1, Ordering::SeqCst);
}

/// Decrement active task count
pub fn decrement_active_tasks() {
    ACTIVE_GOROUTINES.fetch_sub(1, Ordering::SeqCst);
}

#[cfg(feature = "task-runtime")]
pub fn register_task_metrics(metrics: Arc<TaskRuntimeMetrics>) {
    *TASK_METRICS.write() = Some(metrics);
}

#[cfg(feature = "task-runtime")]
pub fn task_metrics_clone() -> Option<Arc<TaskRuntimeMetrics>> {
    TASK_METRICS.read().as_ref().map(Arc::clone)
}

#[cfg(feature = "task-runtime")]
pub fn emit_task_metrics_report() {
    use std::env;

    if env::var_os("OTTER_TASKS_DIAGNOSTICS").is_none() {
        return;
    }

    if let Some(metrics) = task_metrics_clone() {
        let snapshot = metrics.snapshot();
        println!(
            "[tasks] spawned={}, completed={}, waiting={}, channels={}, channel_waiters={}, channel_backlog={}",
            snapshot.tasks_spawned,
            snapshot.tasks_completed,
            snapshot.tasks_waiting,
            snapshot.channels_registered,
            snapshot.channel_waiters,
            snapshot.channel_backlog
        );
    }
}

#[cfg(feature = "task-runtime")]
fn task_metrics_field() -> Option<String> {
    let guard = TASK_METRICS.read();
    guard.as_ref().map(|metrics| {
        let snapshot: TaskMetricsSnapshot = metrics.snapshot();
        format!(
            "\"tasks\":{{\"spawned\":{},\"completed\":{},\"waiting\":{}}},\"channels\":{{\"registered\":{},\"waiting\":{},\"backlog\":{}}}",
            snapshot.tasks_spawned,
            snapshot.tasks_completed,
            snapshot.tasks_waiting,
            snapshot.channels_registered,
            snapshot.channel_waiters,
            snapshot.channel_backlog
        )
    })
}

// ============================================================================
// Symbol Registration
// ============================================================================

fn register_std_runtime_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "runtime.gos".into(),
        symbol: "otter_runtime_gos".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "runtime.cpu_count".into(),
        symbol: "otter_runtime_cpu_count".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "runtime.memory".into(),
        symbol: "otter_runtime_memory".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "runtime.collect_garbage".into(),
        symbol: "otter_runtime_collect_garbage".into(),
        signature: FfiSignature::new(vec![], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "runtime.stats".into(),
        symbol: "otter_runtime_stats".into(),
        signature: FfiSignature::new(vec![], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "runtime.version".into(),
        symbol: "otter_runtime_version".into(),
        signature: FfiSignature::new(vec![], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "runtime.free".into(),
        symbol: "otter_runtime_free_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Unit),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_runtime_symbols,
    }
}
