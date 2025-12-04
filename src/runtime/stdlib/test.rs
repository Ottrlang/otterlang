use std::ffi::CStr;
use std::os::raw::c_char;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};
use abi_stable::std_types::RVec;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TestPoint {
    pub x: i64,
    pub y: i64,
}

/// asserts that a condition is truthy; panic-ing with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert(condition: i64, message: *const c_char) -> i32 {
    if condition != 0 {
        return 0; // Success
    }

    let msg = if message.is_null() {
        "Assertion failed".to_string()
    } else {
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .to_string()
    };

    eprintln!("Assertion failed: {}", msg);
    std::process::exit(1);
}

/// asserts that two strings are equal; panic-ing with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert_eq(
    left: *const c_char,
    right: *const c_char,
    message: *const c_char,
) -> i32 {
    let left_str = unsafe { CStr::from_ptr(left) }
        .to_string_lossy()
        .to_string();
    let right_str = unsafe { CStr::from_ptr(right) }
        .to_string_lossy()
        .to_string();

    if left_str == right_str {
        return 0; // Success
    }

    let msg = if message.is_null() {
        format!(
            "Assertion failed: expected '{}', got '{}'",
            right_str, left_str
        )
    } else {
        let custom_msg = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .to_string();
        format!(
            "{}: expected '{}', got '{}'",
            custom_msg, right_str, left_str
        )
    };

    eprintln!("{}", msg);
    std::process::exit(1);
}

/// asserts that two strings are not equal; panic-ing with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert_ne(
    left: *const c_char,
    right: *const c_char,
    message: *const c_char,
) -> i32 {
    let left_str = unsafe { CStr::from_ptr(left) }
        .to_string_lossy()
        .to_string();
    let right_str = unsafe { CStr::from_ptr(right) }
        .to_string_lossy()
        .to_string();

    if left_str != right_str {
        return 0; // Success
    }

    let msg = if message.is_null() {
        format!(
            "Assertion failed: values should not be equal, but both are '{}'",
            left_str
        )
    } else {
        let custom_msg = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .to_string();
        format!(
            "{}: values should not be equal, but both are '{}'",
            custom_msg, left_str
        )
    };

    eprintln!("{}", msg);
    std::process::exit(1);
}

/// asserts that two floats are approximately equal within `epsilon`; panic-ing
/// with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert_approx_eq(
    left: f64,
    right: f64,
    epsilon: f64,
    message: *const c_char,
) -> i32 {
    let diff = (left - right).abs();
    if diff <= epsilon {
        return 0; // Success
    }

    let msg = if message.is_null() {
        format!(
            "Assertion failed: expected approximately {}, got {} (diff: {}, epsilon: {})",
            right, left, diff, epsilon
        )
    } else {
        let custom_msg = unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .to_string();
        format!(
            "{}: expected approximately {}, got {} (diff: {}, epsilon: {})",
            custom_msg, right, left, diff, epsilon
        )
    };

    eprintln!("{}", msg);
    std::process::exit(1);
}

/// asserts that `condition` is truthy; panic-ing with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert_true(condition: i64, message: *const c_char) -> i32 {
    unsafe { otter_test_assert(condition, message) }
}

/// asserts that `condition` is falsy; panic-ing with `message` otherwise
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_assert_false(condition: i64, message: *const c_char) -> i32 {
    unsafe { otter_test_assert(if condition == 0 { 1 } else { 0 }, message) }
}

/// returns the input point unchanged
///
/// # Safety
///
/// this function crosses the FFI boundary and assumes the TestPoint struct is properly initialized
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_struct_identity(point: TestPoint) -> TestPoint {
    point
}

use once_cell::sync::Lazy;
use std::sync::Mutex;

static SNAPSHOT_STORAGE: Lazy<Mutex<std::collections::HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

/// performs snapshot testing for the given value
///
/// # Safety
///
/// this function dereferences a raw pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn otter_test_snapshot(name: *const c_char, value: *const c_char) -> i32 {
    let name_str = unsafe { CStr::from_ptr(name) }
        .to_string_lossy()
        .to_string();
    let value_str = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .to_string();

    let update_mode = std::env::var("OTTER_UPDATE_SNAPSHOTS").is_ok();

    let mut storage = SNAPSHOT_STORAGE.lock().unwrap();

    if update_mode {
        storage.insert(name_str.clone(), value_str.clone());
        return 0; // Success in update mode
    }

    match storage.get(&name_str) {
        Some(expected) => {
            if expected == &value_str {
                0 // Match
            } else {
                eprintln!("Snapshot mismatch for '{}':", name_str);
                eprintln!("  Expected: {}", expected);
                eprintln!("  Got:      {}", value_str);
                std::process::exit(1);
            }
        }
        None => {
            eprintln!(
                "Snapshot '{}' not found. Run with --update-snapshots to create it.",
                name_str
            );
            eprintln!("  Value: {}", value_str);
            std::process::exit(1);
        }
    }
}

// ============================================================================
// Symbol Registration
// ============================================================================

fn register_std_test_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "test.assert".into(),
        symbol: "otter_test_assert".into(),
        signature: FfiSignature::new(vec![FfiType::I64, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "test.assert_eq".into(),
        symbol: "otter_test_assert_eq".into(),
        signature: FfiSignature::new(vec![FfiType::Str, FfiType::Str, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "test.assert_ne".into(),
        symbol: "otter_test_assert_ne".into(),
        signature: FfiSignature::new(vec![FfiType::Str, FfiType::Str, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "test.assert_approx_eq".into(),
        symbol: "otter_test_assert_approx_eq".into(),
        signature: FfiSignature::new(
            vec![FfiType::F64, FfiType::F64, FfiType::F64, FfiType::Str],
            FfiType::I32,
        ),
    });

    registry.register(FfiFunction {
        name: "test.assert_true".into(),
        symbol: "otter_test_assert_true".into(),
        signature: FfiSignature::new(vec![FfiType::I64, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "test.assert_false".into(),
        symbol: "otter_test_assert_false".into(),
        signature: FfiSignature::new(vec![FfiType::I64, FfiType::Str], FfiType::I32),
    });

    registry.register(FfiFunction {
        name: "test.snapshot".into(),
        symbol: "otter_test_snapshot".into(),
        signature: FfiSignature::new(vec![FfiType::Str, FfiType::Str], FfiType::I32),
    });

    let point_type = FfiType::Struct {
        fields: RVec::from(vec![FfiType::I64, FfiType::I64]),
    };
    registry.register(FfiFunction {
        name: "test.struct_identity".into(),
        symbol: "otter_test_struct_identity".into(),
        signature: FfiSignature::new(vec![point_type.clone()], point_type),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        namespace: "test",
        autoload: false,
        register: register_std_test_symbols,
    }
}
