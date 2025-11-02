use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

// ============================================================================
// JSON Encoding/Decoding
// Simplified JSON implementation - for full support, integrate serde_json
// ============================================================================

#[no_mangle]
pub extern "C" fn otter_std_json_encode(obj: *const c_char) -> *mut c_char {
    if obj.is_null() {
        return std::ptr::null_mut();
    }

    // For now, just pass through the string
    // Full implementation would serialize OtterLang objects to JSON
    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(obj).to_str() {
            // Try to parse as JSON and re-encode for validation
            // For now, assume it's already valid JSON
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
pub extern "C" fn otter_std_json_decode(json_str: *const c_char) -> *mut c_char {
    if json_str.is_null() {
        return std::ptr::null_mut();
    }

    // For now, just pass through the string
    // Full implementation would parse JSON and return OtterLang object representation
    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(json_str).to_str() {
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
pub extern "C" fn otter_std_json_pretty(json_str: *const c_char) -> *mut c_char {
    if json_str.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(json_str).to_str() {
            // Simple pretty-printing (indent with 2 spaces)
            let mut pretty = String::new();
            let mut indent = 0;
            let mut in_string = false;
            let mut escape_next = false;

            for ch in str_ref.chars() {
                if escape_next {
                    pretty.push(ch);
                    escape_next = false;
                    continue;
                }

                if ch == '\\' {
                    escape_next = true;
                    pretty.push(ch);
                    continue;
                }

                if ch == '"' {
                    in_string = !in_string;
                    pretty.push(ch);
                    continue;
                }

                if !in_string {
                    match ch {
                        '{' | '[' => {
                            pretty.push(ch);
                            pretty.push('\n');
                            indent += 1;
                            pretty.push_str(&"  ".repeat(indent));
                        }
                        '}' | ']' => {
                            pretty.push('\n');
                            indent -= 1;
                            pretty.push_str(&"  ".repeat(indent));
                            pretty.push(ch);
                        }
                        ',' => {
                            pretty.push(ch);
                            pretty.push('\n');
                            pretty.push_str(&"  ".repeat(indent));
                        }
                        ':' => {
                            pretty.push(ch);
                            pretty.push(' ');
                        }
                        ' ' | '\n' | '\t' => {
                            // Skip whitespace
                        }
                        _ => {
                            pretty.push(ch);
                        }
                    }
                } else {
                    pretty.push(ch);
                }
            }

            CString::new(pretty)
                .ok()
                .map(CString::into_raw)
                .unwrap_or(std::ptr::null_mut())
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn otter_std_json_validate(json_str: *const c_char) -> bool {
    if json_str.is_null() {
        return false;
    }

    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(json_str).to_str() {
            // Simple JSON validation (check brackets, quotes, etc.)
            let mut stack = Vec::new();
            let mut in_string = false;
            let mut escape_next = false;

            for ch in str_ref.chars() {
                if escape_next {
                    escape_next = false;
                    continue;
                }

                if ch == '\\' {
                    escape_next = true;
                    continue;
                }

                if ch == '"' {
                    in_string = !in_string;
                    continue;
                }

                if !in_string {
                    match ch {
                        '{' => stack.push('}'),
                        '[' => stack.push(']'),
                        '}' | ']' => {
                            if stack.pop() != Some(ch) {
                                return false;
                            }
                        }
                        _ => {}
                    }
                }
            }

            stack.is_empty() && !in_string
        } else {
            false
        }
    }
}

fn register_std_json_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "json.encode".into(),
        symbol: "otter_std_json_encode".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "json.decode".into(),
        symbol: "otter_std_json_decode".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "json.pretty".into(),
        symbol: "otter_std_json_pretty".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "json.validate".into(),
        symbol: "otter_std_json_validate".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Bool),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_json_symbols,
    }
}
