use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};

use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

// ============================================================================
// Built-in Collections Registry
// For lists and maps, we'll use opaque handles
// ============================================================================

type HandleId = u64;
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

fn next_handle_id() -> HandleId {
    NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst)
}

// Simple list storage (for future expansion)
struct List {
    items: Vec<String>, // For now, storing strings
}

static LISTS: Lazy<RwLock<std::collections::HashMap<HandleId, List>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

// Simple map storage (for future expansion)
struct Map {
    items: std::collections::HashMap<String, String>, // For now, string->string
}

static MAPS: Lazy<RwLock<std::collections::HashMap<HandleId, Map>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

// ============================================================================
// Error Handling - Panic and Recovery
// ============================================================================

// Error type representation
struct Error {
    message: String,
}

static ERRORS: Lazy<RwLock<std::collections::HashMap<HandleId, Error>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

// Thread-local panic state
thread_local! {
    static PANIC_STATE: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

// Thread-local defer stack
thread_local! {
    static DEFER_STACK: std::cell::RefCell<Vec<extern "C" fn()>> = std::cell::RefCell::new(Vec::new());
}

// ============================================================================
// len(x) - Get length of string, list, or map
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_len_string(s: *const c_char) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(s).to_str() {
            str_ref.len() as i64
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_len_list(handle: u64) -> i64 {
    let lists = LISTS.read();
    if let Some(list) = lists.get(&handle) {
        list.items.len() as i64
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_len_map(handle: u64) -> i64 {
    let maps = MAPS.read();
    if let Some(map) = maps.get(&handle) {
        map.items.len() as i64
    } else {
        0
    }
}

// ============================================================================
// cap(x) - Get capacity of a list
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_cap_list(handle: u64) -> i64 {
    let lists = LISTS.read();
    if let Some(list) = lists.get(&handle) {
        list.items.capacity() as i64
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_cap_string(s: *const c_char) -> i64 {
    if s.is_null() {
        return 0;
    }
    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(s).to_str() {
            // Strings are immutable, so capacity = length
            str_ref.len() as i64
        } else {
            0
        }
    }
}

// ============================================================================
// append(x, val) - Append to a list
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_append_list_string(handle: u64, val: *const c_char) -> i32 {
    if val.is_null() {
        return 0;
    }

    let val_str = unsafe { CStr::from_ptr(val).to_str().unwrap_or("").to_string() };

    let mut lists = LISTS.write();
    if let Some(list) = lists.get_mut(&handle) {
        list.items.push(val_str);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_append_list_int(handle: u64, val: i64) -> i32 {
    let val_str = val.to_string();
    let mut lists = LISTS.write();
    if let Some(list) = lists.get_mut(&handle) {
        list.items.push(val_str);
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_append_list_float(handle: u64, val: f64) -> i32 {
    let val_str = val.to_string();
    let mut lists = LISTS.write();
    if let Some(list) = lists.get_mut(&handle) {
        list.items.push(val_str);
        1
    } else {
        0
    }
}

// ============================================================================
// delete(map, key) - Delete a key from a map
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_delete_map(handle: u64, key: *const c_char) -> i32 {
    if key.is_null() {
        return 0;
    }

    let key_str = unsafe { CStr::from_ptr(key).to_str().unwrap_or("").to_string() };

    let mut maps = MAPS.write();
    if let Some(map) = maps.get_mut(&handle) {
        if map.items.remove(&key_str).is_some() {
            1
        } else {
            0
        }
    } else {
        0
    }
}

// ============================================================================
// range(start, end) - Generate a range (returns list handle)
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_range_int(start: i64, end: i64) -> u64 {
    let id = next_handle_id();
    let mut items = Vec::new();

    if start <= end {
        for i in start..end {
            items.push(i.to_string());
        }
    }

    let list = List { items };
    LISTS.write().insert(id, list);
    id
}

#[no_mangle]
pub extern "C" fn otter_builtin_range_float(start: f64, end: f64) -> u64 {
    let id = next_handle_id();
    let mut items = Vec::new();

    if start <= end {
        let mut current = start;
        while current < end {
            items.push(current.to_string());
            current += 1.0;
        }
    }

    let list = List { items };
    LISTS.write().insert(id, list);
    id
}

// ============================================================================
// enumerate(list) - Enumerate a list with indices
// Returns a new list handle with "index:value" format
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_enumerate_list(handle: u64) -> u64 {
    let lists = LISTS.read();
    let id = next_handle_id();

    if let Some(list) = lists.get(&handle) {
        let enumerated: Vec<String> = list
            .items
            .iter()
            .enumerate()
            .map(|(idx, val)| format!("{}:{}", idx, val))
            .collect();

        let new_list = List { items: enumerated };
        drop(lists); // Release read lock
        LISTS.write().insert(id, new_list);
    } else {
        // Return empty list if input handle invalid
        let empty_list = List { items: Vec::new() };
        drop(lists);
        LISTS.write().insert(id, empty_list);
    }

    id
}

// ============================================================================
// Helper functions for list/map creation
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_list_new() -> u64 {
    let id = next_handle_id();
    let list = List { items: Vec::new() };
    LISTS.write().insert(id, list);
    id
}

#[no_mangle]
pub extern "C" fn otter_builtin_map_new() -> u64 {
    let id = next_handle_id();
    let map = Map {
        items: std::collections::HashMap::new(),
    };
    MAPS.write().insert(id, map);
    id
}

#[no_mangle]
pub extern "C" fn otter_builtin_list_get(handle: u64, index: i64) -> *mut c_char {
    let lists = LISTS.read();
    if let Some(list) = lists.get(&handle) {
        if index >= 0 && (index as usize) < list.items.len() {
            let val = &list.items[index as usize];
            CString::new(val.clone())
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
pub extern "C" fn otter_builtin_map_get(handle: u64, key: *const c_char) -> *mut c_char {
    if key.is_null() {
        return std::ptr::null_mut();
    }

    let key_str = unsafe { CStr::from_ptr(key).to_str().unwrap_or("").to_string() };

    let maps = MAPS.read();
    if let Some(map) = maps.get(&handle) {
        if let Some(val) = map.items.get(&key_str) {
            CString::new(val.clone())
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
pub extern "C" fn otter_builtin_map_set(
    handle: u64,
    key: *const c_char,
    value: *const c_char,
) -> i32 {
    if key.is_null() || value.is_null() {
        return 0;
    }

    let key_str = unsafe { CStr::from_ptr(key).to_str().unwrap_or("").to_string() };

    let value_str = unsafe { CStr::from_ptr(value).to_str().unwrap_or("").to_string() };

    let mut maps = MAPS.write();
    if let Some(map) = maps.get_mut(&handle) {
        map.items.insert(key_str, value_str);
        1
    } else {
        0
    }
}

// ============================================================================
// panic(msg) - Terminate execution with error message
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_panic(msg: *const c_char) {
    let message = if msg.is_null() {
        "panic: unknown error".to_string()
    } else {
        unsafe {
            CStr::from_ptr(msg)
                .to_str()
                .unwrap_or("panic: invalid error message")
                .to_string()
        }
    };

    // Set panic state in thread-local storage
    PANIC_STATE.with(|state| {
        *state.borrow_mut() = Some(message.clone());
    });

    // Use Rust's panic mechanism
    panic!("{}", message);
}

// ============================================================================
// recover() -> any - Recover from panic
// Returns the panic message if recovering, null otherwise
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_recover() -> *mut c_char {
    PANIC_STATE.with(|state| {
        if let Some(ref msg) = *state.borrow() {
            CString::new(msg.clone())
                .ok()
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        }
    })
}

// ============================================================================
// try(fn) -> (any, Error) - Execute function and return result or error
// Since FFI doesn't support tuples well, we'll use a different approach:
// try returns an opaque handle, and we have separate functions to get result/error
// ============================================================================

struct TryResult {
    result: Option<String>,
    error: Option<HandleId>,
}

static TRY_RESULTS: Lazy<RwLock<std::collections::HashMap<HandleId, TryResult>>> =
    Lazy::new(|| RwLock::new(std::collections::HashMap::new()));

// Function pointer type for try
type TryFn = extern "C" fn() -> *mut c_char;

#[no_mangle]
pub extern "C" fn otter_builtin_try(callback: TryFn) -> u64 {
    let id = next_handle_id();

    // Reset panic state
    PANIC_STATE.with(|state| {
        *state.borrow_mut() = None;
    });

    // Execute function in a catch_unwind block
    let result = catch_unwind(AssertUnwindSafe(|| callback()));

    match result {
        Ok(ptr) => {
            if ptr.is_null() {
                // Function returned null, might be an error
                // Check if panic was set
                let panic_msg = PANIC_STATE.with(|state| state.borrow().clone());

                if let Some(msg) = panic_msg {
                    // Create error handle
                    let error_id = next_handle_id();
                    ERRORS.write().insert(error_id, Error { message: msg });

                    TRY_RESULTS.write().insert(
                        id,
                        TryResult {
                            result: None,
                            error: Some(error_id),
                        },
                    );
                } else {
                    // Null result but no panic - treat as success with null
                    TRY_RESULTS.write().insert(
                        id,
                        TryResult {
                            result: Some(String::new()),
                            error: None,
                        },
                    );
                }
            } else {
                // Success - extract string
                let value = unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("").to_string() };
                unsafe {
                    let _ = CString::from_raw(ptr);
                }

                TRY_RESULTS.write().insert(
                    id,
                    TryResult {
                        result: Some(value),
                        error: None,
                    },
                );
            }
        }
        Err(_) => {
            // Panic occurred
            let panic_msg = PANIC_STATE.with(|state| {
                state
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| "panic: unknown error".to_string())
            });

            let error_id = next_handle_id();
            ERRORS
                .write()
                .insert(error_id, Error { message: panic_msg });

            TRY_RESULTS.write().insert(
                id,
                TryResult {
                    result: None,
                    error: Some(error_id),
                },
            );
        }
    }

    id
}

#[no_mangle]
pub extern "C" fn otter_builtin_try_result(handle: u64) -> *mut c_char {
    let try_results = TRY_RESULTS.read();
    if let Some(try_result) = try_results.get(&handle) {
        if let Some(ref result) = try_result.result {
            CString::new(result.clone())
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
pub extern "C" fn otter_builtin_try_error(handle: u64) -> u64 {
    let try_results = TRY_RESULTS.read();
    if let Some(try_result) = try_results.get(&handle) {
        try_result.error.unwrap_or(0)
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_error_message(error_handle: u64) -> *mut c_char {
    let errors = ERRORS.read();
    if let Some(error) = errors.get(&error_handle) {
        CString::new(error.message.clone())
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    } else {
        std::ptr::null_mut()
    }
}

// ============================================================================
// defer(fn) - Defer function execution until scope exit
// ============================================================================

type DeferFn = extern "C" fn();

#[no_mangle]
pub extern "C" fn otter_builtin_defer(callback: DeferFn) {
    DEFER_STACK.with(|stack| {
        stack.borrow_mut().push(callback);
    });
}

/// Execute all deferred functions (called at scope exit)
/// This should be called by the compiler-generated code
#[no_mangle]
pub extern "C" fn otter_builtin_run_defers() {
    DEFER_STACK.with(|stack| {
        let mut stack_ref = stack.borrow_mut();
        while let Some(callback) = stack_ref.pop() {
            callback();
        }
    });
}

// ============================================================================
// type_of(x) - Get type of a value as string
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_string(_s: *const c_char) -> *mut c_char {
    CString::new("string")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_int(_i: i64) -> *mut c_char {
    CString::new("int")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_float(_f: f64) -> *mut c_char {
    CString::new("float")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_bool(_b: bool) -> *mut c_char {
    CString::new("bool")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_list(_handle: u64) -> *mut c_char {
    CString::new("list")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_map(_handle: u64) -> *mut c_char {
    CString::new("map")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_type_of_opaque(_handle: u64) -> *mut c_char {
    CString::new("opaque")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

// ============================================================================
// fields(obj) - Get fields of an object/struct
// For now, we'll return a JSON string with field information
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_fields(_obj: u64) -> *mut c_char {
    // For now, return empty JSON object
    // Future: track struct definitions and return field list
    CString::new("{}")
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

// ============================================================================
// stringify(x) - Convert value to string
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_int(value: i64) -> *mut c_char {
    CString::new(value.to_string())
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_float(value: f64) -> *mut c_char {
    CString::new(value.to_string())
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_bool(value: bool) -> *mut c_char {
    CString::new(if value { "true" } else { "false" })
        .ok()
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_string(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(s).to_str() {
            CString::new(str_ref)
                .ok()
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_list(handle: u64) -> *mut c_char {
    let lists = LISTS.read();
    if let Some(list) = lists.get(&handle) {
        let items: Vec<String> = list.items.iter().map(|s| format!("\"{}\"", s)).collect();
        let json = format!("[{}]", items.join(", "));
        CString::new(json)
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    } else {
        CString::new("[]")
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }
}

#[no_mangle]
pub extern "C" fn otter_builtin_stringify_map(handle: u64) -> *mut c_char {
    let maps = MAPS.read();
    if let Some(map) = maps.get(&handle) {
        let items: Vec<String> = map
            .items
            .iter()
            .map(|(k, v)| format!("\"{}\": \"{}\"", k, v))
            .collect();
        let json = format!("{{{}}}", items.join(", "));
        CString::new(json)
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    } else {
        CString::new("{}")
            .ok()
            .map(CString::into_raw)
            .unwrap_or(std::ptr::null_mut())
    }
}

// ============================================================================
// select { case ... } - Channel select (simplified version)
// For now, implements a simple select that tries channels in order
// ============================================================================

#[repr(C)]
pub struct SelectCase {
    channel: u64,
    is_send: bool,
    value: *const c_char, // For send operations
}

#[no_mangle]
pub extern "C" fn otter_builtin_select(
    cases: *const SelectCase,
    num_cases: i64,
    default_available: bool,
) -> i64 {
    if cases.is_null() || num_cases <= 0 {
        return -1; // No case selected
    }

    // For now, try cases in order until one succeeds
    // This is a simplified implementation - full select would use polling
    unsafe {
        let cases_slice = std::slice::from_raw_parts(cases, num_cases as usize);

        for (_idx, case) in cases_slice.iter().enumerate() {
            if case.is_send {
                // Try send
                // TODO: integrate with task module when runtime channels are exposed
                // In full implementation, would check channel and send if ready
                continue;
            } else {
                // Try receive
                // TODO: integrate with task module when runtime channels are exposed
                // In full implementation, would check channel and recv if ready
                continue;
            }
        }
    }

    // No case succeeded
    if default_available {
        -1 // Default case
    } else {
        -2 // Block (would block in real implementation)
    }
}

// ============================================================================
// Symbol Registration
// ============================================================================

fn register_builtin_symbols(registry: &SymbolRegistry) {
    // len() functions
    registry.register(FfiFunction {
        name: "len".into(),
        symbol: "otter_builtin_len_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "len<list>".into(),
        symbol: "otter_builtin_len_list".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "len<map>".into(),
        symbol: "otter_builtin_len_map".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    // cap() functions
    registry.register(FfiFunction {
        name: "cap".into(),
        symbol: "otter_builtin_cap_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "cap<list>".into(),
        symbol: "otter_builtin_cap_list".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::I64),
    });

    // append() functions
    registry.register(FfiFunction {
        name: "append<list,string>".into(),
        symbol: "otter_builtin_append_list_string".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "append<list,int>".into(),
        symbol: "otter_builtin_append_list_int".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::I64], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "append<list,float>".into(),
        symbol: "otter_builtin_append_list_float".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::F64], FfiType::I32),
    });

    // delete() function
    registry.register(FfiFunction {
        name: "delete<map>".into(),
        symbol: "otter_builtin_delete_map".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::I32),
    });

    // range() functions
    registry.register(FfiFunction {
        name: "range<int>".into(),
        symbol: "otter_builtin_range_int".into(),
        signature: FfiSignature::new(vec![FfiType::I64, FfiType::I64], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "range<float>".into(),
        symbol: "otter_builtin_range_float".into(),
        signature: FfiSignature::new(vec![FfiType::F64, FfiType::F64], FfiType::Opaque),
    });

    // enumerate() function
    registry.register(FfiFunction {
        name: "enumerate<list>".into(),
        symbol: "otter_builtin_enumerate_list".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Opaque),
    });

    // Helper functions
    registry.register(FfiFunction {
        name: "list.new".into(),
        symbol: "otter_builtin_list_new".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "map.new".into(),
        symbol: "otter_builtin_map_new".into(),
        signature: FfiSignature::new(vec![], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "list.get".into(),
        symbol: "otter_builtin_list_get".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::I64], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "map.get".into(),
        symbol: "otter_builtin_map_get".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque, FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "map.set".into(),
        symbol: "otter_builtin_map_set".into(),
        signature: FfiSignature::new(
            vec![FfiType::Opaque, FfiType::Str, FfiType::Str],
            FfiType::I32,
        ),
    });

    // Error handling functions
    registry.register(FfiFunction {
        name: "panic".into(),
        symbol: "otter_builtin_panic".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "recover".into(),
        symbol: "otter_builtin_recover".into(),
        signature: FfiSignature::new(vec![], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "try".into(),
        symbol: "otter_builtin_try".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "try.result".into(),
        symbol: "otter_builtin_try_result".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "try.error".into(),
        symbol: "otter_builtin_try_error".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Opaque),
    });

    registry.register(FfiFunction {
        name: "error.message".into(),
        symbol: "otter_builtin_error_message".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    // defer() function
    registry.register(FfiFunction {
        name: "defer".into(),
        symbol: "otter_builtin_defer".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Unit),
    });

    // type_of() functions
    registry.register(FfiFunction {
        name: "type_of<string>".into(),
        symbol: "otter_builtin_type_of_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<int>".into(),
        symbol: "otter_builtin_type_of_int".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<float>".into(),
        symbol: "otter_builtin_type_of_float".into(),
        signature: FfiSignature::new(vec![FfiType::F64], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<bool>".into(),
        symbol: "otter_builtin_type_of_bool".into(),
        signature: FfiSignature::new(vec![FfiType::Bool], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<list>".into(),
        symbol: "otter_builtin_type_of_list".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<map>".into(),
        symbol: "otter_builtin_type_of_map".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "type_of<opaque>".into(),
        symbol: "otter_builtin_type_of_opaque".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    // fields() function
    registry.register(FfiFunction {
        name: "fields".into(),
        symbol: "otter_builtin_fields".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    // stringify() functions
    registry.register(FfiFunction {
        name: "stringify<int>".into(),
        symbol: "otter_builtin_stringify_int".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "stringify<float>".into(),
        symbol: "otter_builtin_stringify_float".into(),
        signature: FfiSignature::new(vec![FfiType::F64], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "stringify<bool>".into(),
        symbol: "otter_builtin_stringify_bool".into(),
        signature: FfiSignature::new(vec![FfiType::Bool], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "stringify<string>".into(),
        symbol: "otter_builtin_stringify_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "stringify<list>".into(),
        symbol: "otter_builtin_stringify_list".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "stringify<map>".into(),
        symbol: "otter_builtin_stringify_map".into(),
        signature: FfiSignature::new(vec![FfiType::Opaque], FfiType::Str),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_builtin_symbols,
    }
}
