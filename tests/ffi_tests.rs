use anyhow::Result;

#[test]
fn builtins_are_registered() -> Result<()> {
    let runtime = otterlang::runtime::ffi::Runtime::new()?;
    let symbols = runtime.symbols();

    assert!(symbols.contains("std.io.print"));
    assert!(symbols.contains("std.math.sqrt"));
    assert!(symbols.contains("std.time.now"));

    Ok(())
}
