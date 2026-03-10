// ============================================================
// main.rs — Entry point
// ============================================================
//
// NO #![windows_subsystem = "windows"] — MCP needs stdin/stdout.
// When launched by Claude Code as a child process, stdin/stdout
// are piped so no console window is visible.
// ============================================================

fn main() {
    asqu::run();
}
