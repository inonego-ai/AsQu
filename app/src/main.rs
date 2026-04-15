// ============================================================
// main.rs — Entry point
// ============================================================
//
// windows_subsystem = "windows" suppresses the console window in GUI mode.
// CLI mode reattaches to the parent console via AttachConsole so that
// stdout/stderr still reach the calling terminal.
// ============================================================

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        // GUI mode: no console window (windows_subsystem = "windows")
        asqu::run_gui();
    } else {
        // CLI mode: reattach to the parent process's console so
        // println!/eprintln! output reaches the calling terminal.
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
        }
        asqu::run_cli(args);
    }
}
