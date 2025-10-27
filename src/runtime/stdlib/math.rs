use crate::runtime::symbol_registry::{FfiFunction, FfiSignature, FfiType, SymbolRegistry};

#[no_mangle]
pub extern "C" fn otter_std_math_sqrt(value: f64) -> f64 {
    libm::sqrt(value)
}

#[no_mangle]
pub extern "C" fn otter_std_math_pow(base: f64, exponent: f64) -> f64 {
    libm::pow(base, exponent)
}

#[no_mangle]
pub extern "C" fn otter_std_math_sin(value: f64) -> f64 {
    libm::sin(value)
}

#[no_mangle]
pub extern "C" fn otter_std_math_cos(value: f64) -> f64 {
    libm::cos(value)
}

fn register_std_math_symbols(registry: &SymbolRegistry) {
    registry.register(FfiFunction {
        name: "std.math.sqrt".into(),
        symbol: "otter_std_math_sqrt".into(),
        signature: FfiSignature::new(vec![FfiType::F64], FfiType::F64),
    });

    registry.register(FfiFunction {
        name: "std.math.pow".into(),
        symbol: "otter_std_math_pow".into(),
        signature: FfiSignature::new(vec![FfiType::F64, FfiType::F64], FfiType::F64),
    });

    registry.register(FfiFunction {
        name: "std.math.sin".into(),
        symbol: "otter_std_math_sin".into(),
        signature: FfiSignature::new(vec![FfiType::F64], FfiType::F64),
    });

    registry.register(FfiFunction {
        name: "std.math.cos".into(),
        symbol: "otter_std_math_cos".into(),
        signature: FfiSignature::new(vec![FfiType::F64], FfiType::F64),
    });
}

inventory::submit! {
    crate::runtime::ffi::SymbolProvider {
        register: register_std_math_symbols,
    }
}
