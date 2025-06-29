use super::protocol::handle_rpc;
use super::tools::ToolContext;
use crate::pool::ConnectionPool;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use tiny_http::{Header, Method, Response, Server};

/// Blocking stdio MCP (standalone `sqf mcp`).
pub fn run_stdio(pool: ConnectionPool) -> crate::error::Result<()> {
    let ctx = ToolContext { pool };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Content-Length framed or newline-delimited JSON
        let body: Value = if line.to_ascii_lowercase().starts_with("content-length:") {
            let len: usize = line
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            let mut blank = String::new();
            reader.read_line(&mut blank)?;
            let mut buf = vec![0u8; len];
            reader.read_exact(&mut buf)?;
            serde_json::from_slice(&buf).unwrap_or(json_null())
        } else {
            serde_json::from_str(line).unwrap_or(json_null())
        };

        if body.get("method").and_then(|m| m.as_str()) == Some("notifications/initialized") {
            continue;
        }
        if body.get("id").is_none() && body.get("method").is_some() {
            // notification
            let _ = handle_rpc(&ctx, &body);
            continue;
        }

        let response = handle_rpc(&ctx, &body);
        let payload = serde_json::to_string(&response).unwrap_or_else(|_| "{}".into());
        // Prefer Content-Length framing for MCP clients
        write!(
            stdout,
            "Content-Length: {}\r\n\r\n{}",
            payload.len(),
            payload
        )?;
        stdout.flush()?;
    }
    Ok(())
}

fn json_null() -> Value {
    Value::Null
}

/// Start HTTP MCP listener on `127.0.0.1:port` in a background thread.
/// POST /mcp with JSON-RPC body. Returns stop flag (set true to request shutdown).
pub fn run_http_background(pool: ConnectionPool, port: u16) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    thread::spawn(move || {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        let server = match Server::http(addr) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("mcp: failed to bind {addr}: {err}");
                return;
            }
        };
        eprintln!("mcp: listening on http://127.0.0.1:{port}/mcp");
        let ctx = ToolContext { pool };
        for mut request in server.incoming_requests() {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            if request.method() == &Method::Get && request.url() == "/health" {
                let _ = request.respond(Response::from_string("ok"));
                continue;
            }
            if request.method() != &Method::Post
                || !(request.url() == "/mcp" || request.url().starts_with("/mcp?"))
            {
                let _ = request.respond(Response::from_string("not found").with_status_code(404));
                continue;
            }
            let mut body = String::new();
            let _ = request.as_reader().read_to_string(&mut body);
            let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            if parsed.get("id").is_none()
                && parsed
                    .get("method")
                    .and_then(|m| m.as_str())
                    .is_some_and(|m| m.starts_with("notifications/"))
            {
                let _ = request.respond(Response::empty(204));
                continue;
            }
            let response = handle_rpc(&ctx, &parsed);
            let payload = serde_json::to_string(&response).unwrap_or_else(|_| "{}".into());
            let mut resp = Response::from_string(payload);
            if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
                resp.add_header(h);
            }
            let _ = request.respond(resp);
        }
    });
    stop
}
