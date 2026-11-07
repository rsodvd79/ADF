fn main() {
    if let Err(error) = adf::ui::run_native_smoke() {
        eprintln!("Native smoke failed: {error}");
        std::process::exit(1);
    }
    println!(
        "Native smoke passed: CIAO, blocking dialog, all controls, multiple viewports, CHANGE and Stop."
    );
}
