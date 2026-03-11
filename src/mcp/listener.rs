// mcp/listener.rs — TCP listener for MCP bridge communication

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use super::{McpCommand, McpResponse};

/// Default TCP port for MCP communication.
const DEFAULT_PORT: u16 = 3939;

/// A pending command with its response channel.
pub struct PendingCommand {
    pub command: McpCommand,
    pub responder: mpsc::Sender<McpResponse>,
}

/// Shared command queue between TCP listener and GUI thread.
pub type CommandQueue = Arc<Mutex<VecDeque<PendingCommand>>>;

/// Create a new empty command queue.
pub fn new_command_queue() -> CommandQueue {
    Arc::new(Mutex::new(VecDeque::new()))
}

/// Start the TCP listener on a background thread.
/// The `egui_ctx` is used to wake the GUI thread when commands arrive.
pub fn start(queue: CommandQueue, egui_ctx: egui::Context) -> Option<u16> {
    let port = std::env::var("GEOSCOPE_MCP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("MCP listener failed to bind on port {}: {}", port, e);
            return None;
        }
    };

    log::info!("MCP listener started on 127.0.0.1:{}", port);

    let queue_clone = queue.clone();
    std::thread::Builder::new()
        .name("mcp-listener".to_string())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let q = queue_clone.clone();
                        let ctx = egui_ctx.clone();
                        std::thread::Builder::new()
                            .name("mcp-conn".to_string())
                            .spawn(move || handle_connection(stream, q, ctx))
                            .ok();
                    }
                    Err(e) => {
                        log::error!("MCP accept error: {}", e);
                    }
                }
            }
        })
        .ok();

    Some(port)
}

/// Handle a single TCP connection (one command per line, synchronous).
fn handle_connection(stream: TcpStream, queue: CommandQueue, egui_ctx: egui::Context) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    log::debug!("MCP connection from {}", peer);

    // Set a generous read timeout so we don't block forever
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(300)));

    let reader = BufReader::new(stream.try_clone().unwrap_or_else(|_| {
        // fallback: this shouldn't happen but handle gracefully
        log::error!("MCP: failed to clone stream");
        stream
    }));
    let mut writer = match reader.get_ref().try_clone() {
        Ok(w) => w,
        Err(e) => {
            log::error!("MCP: failed to clone stream for writer: {}", e);
            return;
        }
    };

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let cmd: McpCommand = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(e) => {
                let resp = McpResponse::err(format!("Invalid command JSON: {}", e));
                let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
                continue;
            }
        };

        // Send command to GUI thread and wait for response
        let (tx, rx) = mpsc::channel();
        {
            let mut q = queue.lock().unwrap();
            q.push_back(PendingCommand {
                command: cmd,
                responder: tx,
            });
        }
        // Wake the GUI thread so it processes the command
        egui_ctx.request_repaint();

        // Wait for GUI thread to process (timeout 10s)
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(resp) => {
                let json = serde_json::to_string(&resp).unwrap();
                if writeln!(writer, "{}", json).is_err() {
                    break;
                }
            }
            Err(_) => {
                let resp = McpResponse::err("Command timed out (GUI thread did not respond)");
                let _ = writeln!(writer, "{}", serde_json::to_string(&resp).unwrap());
            }
        }
    }

    log::debug!("MCP connection from {} closed", peer);
}
