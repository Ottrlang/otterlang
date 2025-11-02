use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use std::time::Duration;

use once_cell::sync::Lazy;
use parking_lot::Mutex;

#[cfg(feature = "task-runtime")]
use crate::runtime::stdlib::runtime::task_metrics_clone;
use crate::runtime::stdlib::runtime::{decrement_active_tasks, increment_active_tasks};
use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};
use crate::runtime::task::{runtime, JoinHandle, TaskChannel, TaskRuntimeMetrics};

type HandleId = u64;

type TaskCallback = extern "C" fn();

static TASK_HANDLES: Lazy<Mutex<HashMap<HandleId, JoinHandle>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn next_handle_id() -> HandleId {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn otter_task_spawn(callback: TaskCallback) -> u64 {
    increment_active_tasks();
    let scheduler = runtime().scheduler().clone();
    let join = scheduler.spawn_fn(Some("task.spawn".into()), move || {
        callback();
        decrement_active_tasks();
    });
    let task_id = join.task_id().raw();
    TASK_HANDLES.lock().insert(task_id, join);
    task_id
}

#[no_mangle]
pub extern "C" fn otter_task_join(handle: u64) {
    if let Some(join) = TASK_HANDLES.lock().remove(&handle) {
        join.join();
    }
}

#[no_mangle]
pub extern "C" fn otter_task_detach(handle: u64) {
    TASK_HANDLES.lock().remove(&handle);
}

#[no_mangle]
pub extern "C" fn otter_task_sleep(ms: i64) {
    if ms <= 0 {
        return;
    }
    std::thread::sleep(Duration::from_millis(ms as u64));
}

#[derive(Debug)]
struct ChannelWrapper<T> {
    channel: TaskChannel<T>,
}

macro_rules! channel_registry {
    ($name:ident, $ty:ty) => {
        static $name: Lazy<Mutex<HashMap<HandleId, ChannelWrapper<$ty>>>> =
            Lazy::new(|| Mutex::new(HashMap::new()));
    };
}

channel_registry!(STRING_CHANNELS, String);
channel_registry!(INT_CHANNELS, i64);
channel_registry!(FLOAT_CHANNELS, f64);

#[cfg(feature = "task-runtime")]
fn obtain_metrics() -> Option<Arc<TaskRuntimeMetrics>> {
    task_metrics_clone()
}

#[cfg(not(feature = "task-runtime"))]
fn obtain_metrics() -> Option<Arc<TaskRuntimeMetrics>> {
    None
}

#[no_mangle]
pub extern "C" fn otter_task_channel_string() -> u64 {
    let id = next_handle_id();
    let metrics = obtain_metrics();
    STRING_CHANNELS.lock().insert(
        id,
        ChannelWrapper {
            channel: TaskChannel::with_metrics(metrics),
        },
    );
    id
}

#[no_mangle]
pub extern "C" fn otter_task_channel_int() -> u64 {
    let id = next_handle_id();
    let metrics = obtain_metrics();
    INT_CHANNELS.lock().insert(
        id,
        ChannelWrapper {
            channel: TaskChannel::with_metrics(metrics),
        },
    );
    id
}

#[no_mangle]
pub extern "C" fn otter_task_channel_float() -> u64 {
    let id = next_handle_id();
    let metrics = obtain_metrics();
    FLOAT_CHANNELS.lock().insert(
        id,
        ChannelWrapper {
            channel: TaskChannel::with_metrics(metrics),
        },
    );
    id
}

#[no_mangle]
pub extern "C" fn otter_task_send_string(handle: u64, value: *const c_char) -> i32 {
    if value.is_null() {
        return 0;
    }
    let value = unsafe { CStr::from_ptr(value).to_str().unwrap_or("").to_string() };
    if let Some(wrapper) = STRING_CHANNELS.lock().get(&handle) {
        wrapper.channel.send(value);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_task_send_int(handle: u64, value: i64) -> i32 {
    if let Some(wrapper) = INT_CHANNELS.lock().get(&handle) {
        wrapper.channel.send(value);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_task_send_float(handle: u64, value: f64) -> i32 {
    if let Some(wrapper) = FLOAT_CHANNELS.lock().get(&handle) {
        wrapper.channel.send(value);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_task_recv_string(handle: u64) -> *mut c_char {
    if let Some(wrapper) = STRING_CHANNELS.lock().get(&handle) {
        if let Some(value) = wrapper.channel.recv() {
            return CString::new(value)
                .ok()
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut());
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn otter_task_recv_int(handle: u64) -> i64 {
    if let Some(wrapper) = INT_CHANNELS.lock().get(&handle) {
        return wrapper.channel.recv().unwrap_or(0);
    }
    0
}

#[no_mangle]
pub extern "C" fn otter_task_recv_float(handle: u64) -> f64 {
    if let Some(wrapper) = FLOAT_CHANNELS.lock().get(&handle) {
        return wrapper.channel.recv().unwrap_or(0.0);
    }
    0.0
}

#[no_mangle]
pub extern "C" fn otter_task_close_channel(handle: u64) {
    STRING_CHANNELS.lock().remove(&handle);
    INT_CHANNELS.lock().remove(&handle);
    FLOAT_CHANNELS.lock().remove(&handle);
}

fn register_std_task_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "task.spawn".into(),
        symbol: "otter_task_spawn".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.join".into(),
        symbol: "otter_task_join".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "task.detach".into(),
        symbol: "otter_task_detach".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "task.sleep".into(),
        symbol: "otter_task_sleep".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "task.channel<string>".into(),
        symbol: "otter_task_channel_string".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.channel<int>".into(),
        symbol: "otter_task_channel_int".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.channel<float>".into(),
        symbol: "otter_task_channel_float".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.send<string>".into(),
        symbol: "otter_task_send_string".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.send<int>".into(),
        symbol: "otter_task_send_int".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::I64], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.send<float>".into(),
        symbol: "otter_task_send_float".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::F64], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.recv<string>".into(),
        symbol: "otter_task_recv_string".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "task.recv<int>".into(),
        symbol: "otter_task_recv_int".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "task.recv<float>".into(),
        symbol: "otter_task_recv_float".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::F64),
    });

    registry.register(FfiFunction {
        name: "task.close".into(),
        symbol: "otter_task_close_channel".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Unit),
    });

    // Register convenience aliases for underscore notation
    registry.register(FfiFunction {
        name: "task.channel_int".into(),
        symbol: "otter_task_channel_int".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.channel_float".into(),
        symbol: "otter_task_channel_float".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.channel_string".into(),
        symbol: "otter_task_channel_string".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "task.send_int".into(),
        symbol: "otter_task_send_int".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::I64], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.send_float".into(),
        symbol: "otter_task_send_float".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::F64], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.send_string".into(),
        symbol: "otter_task_send_string".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "task.recv_int".into(),
        symbol: "otter_task_recv_int".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "task.recv_float".into(),
        symbol: "otter_task_recv_float".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::F64),
    });

    registry.register(FfiFunction {
        name: "task.recv_string".into(),
        symbol: "otter_task_recv_string".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_task_symbols,
    }
}
