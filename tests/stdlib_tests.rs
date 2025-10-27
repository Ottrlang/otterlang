#[test]
fn math_functions_behave() {
    let sqrt = otterlang::runtime::stdlib::math::otter_std_math_sqrt(16.0);
    assert!((sqrt - 4.0).abs() < 1e-6);

    let pow = otterlang::runtime::stdlib::math::otter_std_math_pow(2.0, 3.0);
    assert!((pow - 8.0).abs() < 1e-6);

    let sin = otterlang::runtime::stdlib::math::otter_std_math_sin(std::f64::consts::PI / 2.0);
    assert!((sin - 1.0).abs() < 1e-6);

    let cos = otterlang::runtime::stdlib::math::otter_std_math_cos(0.0);
    assert!((cos - 1.0).abs() < 1e-6);
}

#[test]
fn time_now_positive() {
    let now = otterlang::runtime::stdlib::time::otter_std_time_now_ms();
    assert!(now > 0);
}

#[test]
fn system_information_available() {
    let cores = otterlang::runtime::stdlib::sys::otter_std_sys_cores();
    assert!(cores >= 1);

    let total_mem = otterlang::runtime::stdlib::sys::otter_std_sys_total_memory_bytes();
    assert!(total_mem > 0);
}
