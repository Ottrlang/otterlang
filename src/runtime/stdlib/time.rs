use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

// ============================================================================
// Time and Duration Structures
// ============================================================================

type HandleId = u64;
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

fn next_handle_id() -> HandleId {
    NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst)
}

struct Time {
    epoch_ms: i64,
}

struct DurationHandle {
    ms: i64,
}

static TIMES: Lazy<RwLock<std::collections::HashMap<HandleId, Time>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

static DURATIONS: Lazy<RwLock<std::collections::HashMap<HandleId, DurationHandle>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

#[no_mangle]
pub extern "C" fn otter_std_time_now() -> u64 {
    let id = next_handle_id();
    let now = chrono::Utc::now().timestamp_millis();
    let time = Time { epoch_ms: now };
    TIMES.write().insert(id, time);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_time_now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[no_mangle]
pub extern "C" fn otter_std_time_sleep_ms(milliseconds: i64) {
    if milliseconds <= 0 {
        return;
    }
    thread::sleep(Duration::from_millis(milliseconds as u64));
}

#[no_mangle]
pub extern "C" fn otter_std_time_since(t: u64) -> u64 {
    let times = TIMES.read();
    if let Some(start_time) = times.get(&t) {
        let now = chrono::Utc::now().timestamp_millis();
        let duration_ms = now - start_time.epoch_ms;

        let id = next_handle_id();
        let duration = DurationHandle { ms: duration_ms };
        drop(times);
        DURATIONS.write().insert(id, duration);
        id
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_std_time_format(t: u64, fmt: *const c_char) -> *mut c_char {
    if fmt.is_null() {
        return std::ptr::null_mut();
    }

    let format_str = unsafe {
        CStr::from_ptr(fmt)
            .to_str()
            .unwrap_or("%Y-%m-%d %H:%M:%S")
            .to_string()
    };

    let times = TIMES.read();
    if let Some(time) = times.get(&t) {
        let dt = chrono::DateTime::from_timestamp_millis(time.epoch_ms);
        if let Some(dt) = dt {
            let formatted = dt.format(&format_str).to_string();
            CString::new(formatted)
                .ok()
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        }
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn otter_std_time_parse(fmt: *const c_char, text: *const c_char) -> u64 {
    if fmt.is_null() || text.is_null() {
        return 0;
    }

    let format_str = unsafe {
        CStr::from_ptr(fmt)
            .to_str()
            .unwrap_or("%Y-%m-%d %H:%M:%S")
            .to_string()
    };

    let text_str = unsafe { CStr::from_ptr(text).to_str().unwrap_or("").to_string() };

    // Try to parse using chrono
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&text_str, &format_str) {
        let epoch_ms = dt.and_utc().timestamp_millis();
        let id = next_handle_id();
        let time = Time { epoch_ms };
        TIMES.write().insert(id, time);
        id
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_std_time_tick(ms: i64) -> u64 {
    let id = next_handle_id();
    let time = Time { epoch_ms: ms };
    TIMES.write().insert(id, time);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_time_after(ms: i64) -> u64 {
    let id = next_handle_id();
    let now = chrono::Utc::now().timestamp_millis();
    let time = Time { epoch_ms: now + ms };
    TIMES.write().insert(id, time);
    id
}

#[no_mangle]
pub extern "C" fn otter_std_time_epoch_ms(t: u64) -> i64 {
    let times = TIMES.read();
    if let Some(time) = times.get(&t) {
        time.epoch_ms
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_std_duration_ms(d: u64) -> i64 {
    let durations = DURATIONS.read();
    if let Some(duration) = durations.get(&d) {
        duration.ms
    } else {
        0
    }
}

fn register_std_time_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "time.now".into(),
        symbol: "otter_std_time_now".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "std.time.now".into(),
        symbol: "otter_std_time_now_ms".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "time.sleep".into(),
        symbol: "otter_std_time_sleep_ms".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "time.since".into(),
        symbol: "otter_std_time_since".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "time.format".into(),
        symbol: "otter_std_time_format".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "time.parse".into(),
        symbol: "otter_std_time_parse".into(),
        signature: FfiSignature::new(vec![FfiType::Str, FfiType::Str], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "time.tick".into(),
        symbol: "otter_std_time_tick".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "time.after".into(),
        symbol: "otter_std_time_after".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "time.epoch_ms".into(),
        symbol: "otter_std_time_epoch_ms".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "duration.ms".into(),
        symbol: "otter_std_duration_ms".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    // Convenience aliases
    registry.register(FfiFunction {
        name: "time.now_ms".into(),
        symbol: "otter_std_time_now_ms".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "time.now_us".into(),
        symbol: "otter_std_time_now_us".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "time.now_ns".into(),
        symbol: "otter_std_time_now_ns".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "time.now_sec".into(),
        symbol: "otter_std_time_now_sec".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_time_symbols,
    }
}
