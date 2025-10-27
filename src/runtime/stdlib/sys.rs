use sysinfo::System;

use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

#[no_mangle]
pub extern "C" fn otter_std_sys_cores() -> i64 {
    let mut system = System::new_all();
    system.refresh_cpu();
    system.cpus().len() as i64
}

#[no_mangle]
pub extern "C" fn otter_std_sys_total_memory_bytes() -> i64 {
    let mut system = System::new_all();
    system.refresh_memory();
    (system.total_memory() * 1024) as i64
}

#[no_mangle]
pub extern "C" fn otter_std_sys_available_memory_bytes() -> i64 {
    let mut system = System::new_all();
    system.refresh_memory();
    (system.available_memory() * 1024) as i64
}

fn register_std_sys_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "std.sys.cores".into(),
        symbol: "otter_std_sys_cores".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "std.sys.total_memory".into(),
        symbol: "otter_std_sys_total_memory_bytes".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });

    registry.register(FfiFunction {
        name: "std.sys.available_memory".into(),
        symbol: "otter_std_sys_available_memory_bytes".into(),
        signature: FfiSignature::new(vec![], FfiType::I64),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_sys_symbols,
    }
}
