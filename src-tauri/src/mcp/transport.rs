// ============================================================
// mcp/transport.rs — stdio transport setup
// ============================================================

use rmcp::{ServiceExt, transport::stdio};
use tracing::{error, info};

use crate::state::SharedState;

use super::tools::AsQuMcpServer;

// ------------------------------------------------------------
// Start the MCP server on stdin/stdout
// ------------------------------------------------------------
pub async fn start_mcp_server(
    state: SharedState,
    session_id: String,
    session_name: String,
    session_cwd: String,
) {
    info!("MCP server starting on stdio");

    let handler = AsQuMcpServer::new(
        state.clone(),
        session_id,
        session_name,
        session_cwd,
    );

    match handler.serve(stdio()).await {
        Ok(service) => {
            info!("MCP server connected");

            // Block until stdin closes (Claude Code disconnects)
            let _ = service.waiting().await;

            info!("MCP stdin closed — exiting");
            // Process will exit when Tauri event loop ends
            std::process::exit(0);
        }
        Err(e) => {
            error!("MCP server failed to start: {:?}", e);
        }
    }
}
