//! Tool gate + real read-only HTTP GET after ALLOW.
//!
//! - `http_get` on allow-list → network GET (body truncated)
//! - deny-list tools → no network, audited
//!
//! ```bash
//! cargo run --example tool_gate_http
//! ```

use omega_cognitive_kernel::types::Action;
use omega_cognitive_kernel::{AuditLog, Executor, Result, ToolGateExecutor};
use std::collections::HashMap;
use std::env::temp_dir;
use std::path::PathBuf;

fn tool(name: &str, url: Option<&str>) -> Action {
    let mut params = HashMap::new();
    if let Some(u) = url {
        params.insert("url".into(), serde_json::json!(u));
    }
    Action {
        name: name.into(),
        params,
        constraints_checked: true,
    }
}

fn main() -> Result<()> {
    let root: PathBuf = temp_dir().join(format!("omega-tool-http-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).ok();
    let state_marker = root.join("state.json");
    let audit_path = AuditLog::beside_state(&state_marker).path;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Tool gate + real http_get (GET only)            ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("audit: {}", audit_path.display());
    println!();

    let mut gate = ToolGateExecutor::new(["http_get".into(), "search_docs".into()])
        .deny([
            "delete_database".into(),
            "shell_exec".into(),
            "http_post".into(), // POST not implemented — deny by policy
        ])
        .with_audit(AuditLog {
            path: audit_path.clone(),
        })
        .enable_http_get(true)
        .fail_closed(false)
        .echo(true);

    // Public, stable endpoint (example.com).
    let proposals = [
        tool("http_get", Some("https://example.com")),
        tool("delete_database", None), // DENY — no network
        tool("search_docs", None),     // ALLOW simulated
        tool("http_get", Some("file:///etc/passwd")), // should fail scheme check if reached
    ];

    println!("── proposals ──");
    // Run one-by-one so a scheme error on last doesn't hide earlier results.
    for p in &proposals {
        match gate.execute(std::slice::from_ref(p)) {
            Ok(rows) => {
                for r in rows {
                    let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
                    let action = r.get("action").and_then(|v| v.as_str()).unwrap_or("?");
                    if status == "ok" && r.get("http_status").is_some() {
                        println!(
                            "  OK  {action} http={} truncated={:?} preview_len={}",
                            r["http_status"],
                            r.get("truncated"),
                            r.get("body_preview")
                                .and_then(|v| v.as_str())
                                .map(|s| s.len())
                                .unwrap_or(0)
                        );
                    } else {
                        println!("  {status} {action} {r}");
                    }
                }
            }
            Err(e) => println!("  ERR {e}"),
        }
    }

    let sum = AuditLog {
        path: audit_path.clone(),
    }
    .summarize()?;
    println!("\n── audit ──");
    println!(
        "allowed={} denied={} events={}",
        sum.tools_allowed, sum.tools_denied, sum.total_events
    );

    println!("\n── takeaway ──");
    println!("1. DENY paths never open sockets.");
    println!("2. ALLOW http_get can perform a real GET (truncated body).");
    println!("3. Still not a full LLM proxy — one read-only tool behind the gate.");

    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}
