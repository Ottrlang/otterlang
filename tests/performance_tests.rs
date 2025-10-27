use std::time::Duration;

#[test]
fn profiler_records_named_phases() {
    let mut profiler = otterlang::utils::profiler::Profiler::new();
    profiler.record_phase("dummy", || std::thread::sleep(Duration::from_millis(1)));
    assert_eq!(profiler.phases().len(), 1);
    assert_eq!(profiler.phases()[0].name, "dummy");
}
