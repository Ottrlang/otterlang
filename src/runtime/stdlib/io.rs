use std::ffi::{CStr, CString};
use std::io::{self, BufRead, Write};
use std::os::raw::c_char;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

#[no_mangle]
pub extern "C" fn otter_std_io_print(message: *const c_char) {
    if message.is_null() {
        return;
    }

    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(message).to_str() {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(str_ref.as_bytes());
            let _ = stdout.flush();
        }
    }
}

#[no_mangle]
pub extern "C" fn otter_std_io_println(message: *const c_char) {
    if message.is_null() {
        println!();
        return;
    }

    unsafe {
        if let Ok(str_ref) = CStr::from_ptr(message).to_str() {
            println!("{str_ref}");
        }
    }
}

#[no_mangle]
pub extern "C" fn otter_std_io_read_line() -> *mut c_char {
    let mut line = String::new();
    let mut stdin = io::stdin().lock();
    match stdin.read_line(&mut line) {
        Ok(0) => std::ptr::null_mut(),
        Ok(_) => {
            let trimmed = line.trim_end_matches(['\n', '\r']).to_string();
            CString::new(trimmed)
                .map(CString::into_raw)
                .unwrap_or_else(|_| std::ptr::null_mut())
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn otter_std_io_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

fn register_std_io_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "std.io.print".into(),
        symbol: "otter_std_io_print".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "std.io.println".into(),
        symbol: "otter_std_io_println".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Unit),
    });

    registry.register(FfiFunction {
        name: "std.io.read_line".into(),
        symbol: "otter_std_io_read_line".into(),
        signature: FfiSignature::new(vec![], FfiType::Str),
    });

    registry.register(FfiFunction {
        name: "std.io.free".into(),
        symbol: "otter_std_io_free_string".into(),
        signature: FfiSignature::new(vec![FfiType::Str], FfiType::Unit),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_io_symbols,
    }
}
