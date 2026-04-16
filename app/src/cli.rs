// ============================================================
// cli.rs — CLI client mode (InoCLI routing + InoIPC client)
// ============================================================

use std::process::Command;

use inocli::{ArgParser, CommandArgs, CommandInfo, CommandRegistry};
use inoipc::IpcConnection;
use inoipc::transport::NamedPipeTransport;

use crate::ipc::types::{AskItem, IpcRequest};
use crate::ipc::server::PIPE_NAME;

// ============================================================
// Entry point for CLI mode
// ============================================================

pub fn run_cli(args: Vec<String>) {
    // Parse arguments with InoCLI
    let str_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let mut parser = ArgParser::new();
    let parsed = match parser.parse(&str_refs) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("asqu: parse error: {e}");
            std::process::exit(1);
        }
    };

    // Route to the matching command
    let registry = build_registry();
    let (info, cmd_args) = match registry.resolve(&parsed) {
        Ok(r) => r,
        Err(_) => {
            eprintln!("asqu: unknown command. Usage: asqu <ask|wait|get|dismiss|open> [args]");
            std::process::exit(1);
        }
    };

    let session_id = get_session_id(&parsed);

    // Build the IPC request JSON
    let request = match build_request(info.key.as_str(), &cmd_args, &session_id) {
        Some(r) => r,
        None => std::process::exit(1),
    };

    let req_json = match serde_json::to_string(&request) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("asqu: serialization error: {e}");
            std::process::exit(1);
        }
    };

    // Connect and send.
    // wait: 5-minute timeout (blocks until answered). Others: 5-second timeout.
    // After spawning GUI, use a longer timeout (15s) to allow Tauri + WebView2 startup.
    let post_spawn_ms: u64 = if info.key == "wait" { 300_000 } else { 15_000 };
    let mut conn = IpcConnection::new(NamedPipeTransport::new(PIPE_NAME));

    // Try connecting quickly first (500ms). If the server isn't up yet, spawn GUI
    // and retry with the post-spawn timeout. This avoids the race condition caused
    // by exists() consuming a pipe connection slot.
    let response = match conn.request_with_retry(&req_json, 500, 50) {
        Ok(r) => r,
        Err(_) => {
            spawn_gui();
            match conn.request_with_retry(&req_json, post_spawn_ms, 50) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("asqu: IPC error: {e}");
                    std::process::exit(1);
                }
            }
        }
    };

    println!("{}", response.raw_json());
}

// ============================================================
// Build InoCLI command registry
// ============================================================

fn build_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(CommandInfo::new(
        vec!["ask".to_string()],
        "Ask one or more questions",
        |_| {},
    ));
    registry.register(CommandInfo::new(
        vec!["wait".to_string()],
        "Wait for answers to given question IDs",
        |_| {},
    ));
    registry.register(CommandInfo::new(
        vec!["get".to_string()],
        "Get current answer state (non-blocking)",
        |_| {},
    ));
    registry.register(CommandInfo::new(
        vec!["dismiss".to_string()],
        "Dismiss pending questions",
        |_| {},
    ));
    registry.register(CommandInfo::new(
        vec!["open".to_string()],
        "Show the AsQu window",
        |_| {},
    ));
    registry.register(CommandInfo::new(
        vec!["shutdown".to_string()],
        "Gracefully shut down the AsQu server",
        |_| {},
    ));

    registry
}

// ============================================================
// Build IpcRequest from parsed CLI args
// ============================================================

fn build_request(command: &str, args: &CommandArgs, session_id: &str) -> Option<IpcRequest> {
    match command {
        "ask" => {
            let questions = parse_ask_items(args)?;
            let display_name = read_ai_title(session_id);
            Some(IpcRequest::Ask {
                session_id: session_id.to_string(),
                display_name,
                questions,
            })
        }

        "wait" => {
            let ids = parse_ids(args);
            let require_all = !args.flag("any");
            let timeout_secs = args.get_int_opt_or("timeout", 0);
            let timeout_secs = if timeout_secs > 0 {
                Some(timeout_secs as u64)
            } else {
                None
            };
            Some(IpcRequest::Wait {
                session_id: session_id.to_string(),
                ids,
                require_all,
                timeout_secs,
            })
        }

        "get" => {
            let ids = parse_ids(args);
            Some(IpcRequest::Get {
                session_id: session_id.to_string(),
                ids,
            })
        }

        "dismiss" => {
            let ids = parse_ids(args);
            let reason = args.opt("reason").map(|s| s.to_string());
            Some(IpcRequest::Dismiss {
                session_id: session_id.to_string(),
                ids,
                reason,
            })
        }

        "open" => Some(IpcRequest::OpenUi),

        "shutdown" => Some(IpcRequest::Shutdown),

        _ => {
            eprintln!("asqu: unknown command: {command}");
            None
        }
    }
}

// ============================================================
// Parse ask items from CLI args
// ============================================================

fn parse_ask_items(args: &CommandArgs) -> Option<Vec<AskItem>> {
    let Some(first) = args.get(0) else {
        eprintln!("asqu ask: argument must be a JSON array (e.g. '[{{\"text\":\"Q?\",\"choices\":[\"A\",\"B\"]}}]')");
        return None;
    };

    match serde_json::from_str::<Vec<AskItem>>(first) {
        Ok(items) => Some(items),
        Err(e) => {
            eprintln!("asqu ask: argument must be a JSON array: {e}");
            None
        }
    }
}

// ------------------------------------------------------------
// Parse question IDs from positional args (space or comma separated)
// ------------------------------------------------------------
fn parse_ids(args: &CommandArgs) -> Vec<String> {
    let mut ids = Vec::new();
    for pos in &args.positionals {
        // Support comma-separated IDs in a single arg
        for id in pos.split(',') {
            let id = id.trim();
            if !id.is_empty() {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

// ============================================================
// ai-title reading from Claude Code transcript JSONL
// ============================================================

/// Try to read the ai-title from the Claude Code transcript JSONL for this session.
/// All I/O errors are silently swallowed — callers must handle None gracefully.
fn read_ai_title(session_id: &str) -> Option<String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;

    let cwd = std::env::current_dir().ok()?;
    let project_dir = encode_cwd_to_project_dir(&cwd)?;

    let jsonl_path = std::path::Path::new(&home)
        .join(".claude")
        .join("projects")
        .join(&project_dir)
        .join(format!("{}.jsonl", session_id));

    let content = std::fs::read_to_string(&jsonl_path).ok()?;

    // Return the last ai-title entry (Claude may revise it over the conversation)
    content
        .lines()
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            if v.get("type")?.as_str()? == "ai-title" {
                v.get("aiTitle")?.as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .last()
}

/// Encode current working directory to the Claude Code project directory name.
/// Rule: lowercase drive + "--" + rest where "/." → "--", "/" → "-", " " → "-".
fn encode_cwd_to_project_dir(cwd: &std::path::Path) -> Option<String> {
    let cwd_str = cwd.to_string_lossy();
    let normalized = cwd_str.replace('\\', "/");

    let encoded = if let Some(colon_pos) = normalized.find(':') {
        let drive = normalized[..colon_pos].to_lowercase();
        let rest = &normalized[colon_pos + 1..];
        let rest_encoded = rest
            .replace("/.", "--")
            .replace('/', "-")
            .replace(' ', "-");
        let rest_trimmed = rest_encoded.trim_start_matches('-').to_string();
        format!("{}--{}", drive, rest_trimmed)
    } else {
        normalized
            .replace("/.", "--")
            .replace('/', "-")
            .replace(' ', "-")
            .trim_start_matches('-')
            .to_string()
    };

    if encoded.is_empty() { None } else { Some(encoded) }
}

// ============================================================
// Session ID resolution from environment variables
// ============================================================

fn get_session_id(args: &CommandArgs) -> String {
    // Explicit override via --session-id
    if let Some(id) = args.opt("session-id") {
        return id.to_string();
    }

    // Try environment variables in priority order
    for var in &["CLAUDE_SESSION_ID", "CLAUDE_CODE_SESSION_ID", "ANTHROPIC_SESSION_ID"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                return val;
            }
        }
    }

    // Fallback: generate a new UUID
    uuid::Uuid::new_v4().to_string()
}

// ============================================================
// Spawn the GUI process in the background (detached, no console)
// ============================================================

fn spawn_gui() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("asqu: cannot determine executable path: {e}");
            std::process::exit(1);
        }
    };

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let _ = Command::new(&exe)
            .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
            .spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = Command::new(&exe).spawn();
    }
}
