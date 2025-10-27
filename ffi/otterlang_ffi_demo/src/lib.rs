use abi_stable::std_types::{RString, RVec};
use otterlang::runtime::ffi::{StableExportSet, StableFunction};
use otterlang::runtime::symbol_registry::{FfiType};

#[no_mangle]
pub extern "C" fn otterlang_exports() -> StableExportSet {
    println!("Otter FFI demo loaded!");

    let functions = RVec::from(vec![StableFunction {
        name: RString::from("ffi.otterlang_ffi_demo.add"),
        symbol: RString::from("add"),
        params: RVec::from(vec![FfiType::F64, FfiType::F64]),
        result: FfiType::F64,
    }]);

    StableExportSet { functions }
}

#[no_mangle]
pub extern "C" fn add(a: f64, b: f64) -> f64 {
    a + b
}
