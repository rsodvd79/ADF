fn main() {
    let result = if std::env::args().any(|arg| arg == "--runtime-only") {
        adf::ui::run_native_runner_smoke()
    } else {
        adf::ui::run_native_smoke()
    };
    if let Err(error) = result {
        eprintln!("Native smoke failed: {error}");
        std::process::exit(1);
    }
    println!(
        "Native smoke passed: CIAO, blocking dialog, all controls, multiple viewports, CHANGE and Stop."
    );
}
