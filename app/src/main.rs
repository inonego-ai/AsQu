// ============================================================
// main.rs — Entry point
// ============================================================
//
// CLI mode must keep normal inherited stdout/stderr handles so agents that
// capture process output (including Codex) can read JSON responses.
// GUI mode detaches from the console immediately on Windows to avoid leaving
// an extra terminal window behind when launched directly.
// ============================================================

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::Console::FreeConsole();
        }
        asqu::run_gui();
    } else {
        asqu::run_cli(args);
    }
}
