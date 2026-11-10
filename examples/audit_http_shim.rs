//! Minimal **localhost** read-only audit HTTP shim.
//!
//! Not a product gateway. Proves an external process can pull
//! `AuditSummary` over HTTP without Axum/auth/multi-tenant complexity.
//!
//! ```bash
//! # terminal 1 — leave running
//! cargo run --example audit_http_shim -- ./path/to/state.json
//! # or generate a short demo trail then serve:
//! cargo run --example audit_http_shim
//!
//! # terminal 2
//! curl -s http://127.0.0.1:8787/health
//! curl -s http://127.0.0.1:8787/audit/summary | head
//! ```
//!
//! Bind: `127.0.0.1` only. No TLS, no auth — demo use only.

use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::{Kernel, Result};
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

const DEFAULT_PORT: u16 = 8787;

fn main() -> Result<()> {
    let state = resolve_state_path()?;
    let audit = AuditLog::beside_state(&state);
    let port: u16 = env::var("OMEGA_AUDIT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let addr = format!("127.0.0.1:{port}");

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Omega audit HTTP shim (localhost, read-only)    ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("state : {}", state.display());
    println!("audit : {}", audit.path.display());
    println!("listen: http://{addr}");
    println!("routes: GET /health  GET /audit/summary  GET /audit/tail?n=20");
    println!("(Ctrl+C to stop)\n");

    let listener = TcpListener::bind(&addr).map_err(|e| {
        omega_cognitive_kernel::OmegaError::Persist(format!("bind {addr}: {e}"))
    })?;

    for conn in listener.incoming() {
        let mut stream = match conn {
            Ok(s) => s,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let line = req.lines().next().unwrap_or("");
        let (method, path) = parse_request_line(line);

        let (status, content_type, body) = match (method.as_str(), path.as_str()) {
            ("GET", "/health") => (
                200,
                "application/json",
                r#"{"ok":true,"service":"omega-audit-shim"}"#.to_string(),
            ),
            ("GET", p) if p == "/audit/summary" || p.starts_with("/audit/summary?") => {
                match audit.summarize() {
                    Ok(s) => match serde_json::to_string_pretty(&s) {
                        Ok(j) => (200, "application/json", j),
                        Err(e) => (500, "text/plain", format!("serialize: {e}")),
                    },
                    Err(e) => (500, "text/plain", format!("summarize: {e}")),
                }
            }
            ("GET", p) if p.starts_with("/audit/tail") => {
                let n = query_param(p, "n").and_then(|s| s.parse().ok()).unwrap_or(20);
                match audit.tail(n) {
                    Ok(rows) => match serde_json::to_string_pretty(&rows) {
                        Ok(j) => (200, "application/json", j),
                        Err(e) => (500, "text/plain", format!("serialize: {e}")),
                    },
                    Err(e) => (500, "text/plain", format!("tail: {e}")),
                }
            }
            ("GET", "/") => (
                200,
                "text/plain",
                "omega audit shim\nGET /health\nGET /audit/summary\nGET /audit/tail?n=20\n".into(),
            ),
            _ => (404, "text/plain", "not found\n".into()),
        };

        let resp = format!(
            "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            reason(status),
            body.len(),
        );
        let _ = stream.write_all(resp.as_bytes());
    }
    Ok(())
}

fn resolve_state_path() -> Result<PathBuf> {
    if let Some(p) = env::args().nth(1) {
        return Ok(PathBuf::from(p));
    }
    // Generate a short trail so /audit/summary is non-empty out of the box.
    let path = env::temp_dir().join(format!("omega-shim-state-{}.json", uuid::Uuid::new_v4()));
    let mut k = Kernel::with_state_path(&path);
    k.autosave = true;
    k.auto_checkpoint_on_patch = true;
    let _ = k.run("shim demo trail", 5)?;
    k.save()?;
    println!("(no state arg — wrote demo trail at {})", path.display());
    Ok(path)
}

fn parse_request_line(line: &str) -> (String, String) {
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    (method, path)
}

fn query_param<'a>(path: &'a str, key: &str) -> Option<&'a str> {
    let q = path.split_once('?')?.1;
    for pair in q.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return Some(v);
            }
        }
    }
    None
}

fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

#[allow(dead_code)]
fn _path_exists(p: &Path) -> bool {
    p.exists()
}
