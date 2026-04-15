// ============================================================
// ipc/server.rs — Named Pipe IPC server
// ============================================================

use std::sync::Arc;
use std::sync::Mutex;

use inoipc::transport::NamedPipeServer;

use crate::state::AppState;

use super::handlers::handle_connection;

/// Pipe name (without \\.\pipe\ prefix — InoIPC adds it)
pub const PIPE_NAME: &str = "asqu";

// -----------------------------------------------------------------------
// Start the Named Pipe IPC server in a dedicated std::thread.
// Accepts one connection per thread (each client gets its own thread).
// -----------------------------------------------------------------------
pub fn start_ipc_server(state: Arc<Mutex<AppState>>) {
    std::thread::Builder::new()
        .name("ipc-server".to_string())
        .spawn(move || {
            let server = NamedPipeServer::new(PIPE_NAME);
            let _ = server.start(move |conn| {
                let state = Arc::clone(&state);
                std::thread::Builder::new()
                    .name("ipc-client".to_string())
                    .spawn(move || handle_connection(conn, state))
                    .ok();
            });
        })
        .expect("failed to spawn ipc-server thread");
}
