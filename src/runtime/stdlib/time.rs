use std::thread;
use std::time::Duration;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

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

fn register_std_time_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "std.time.now".into(),
        symbol: "otter_std_time_now_ms".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "std.time.sleep".into(),
        symbol: "otter_std_time_sleep_ms".into(),
        signature: FfiSignature::new(vec![FfiType::I64], FfiType::Unit),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_time_symbols,
    }
}
