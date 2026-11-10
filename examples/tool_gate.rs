//! Minimal tool-call gate demo (safety MVP story).
//!
//! A *simulated* model proposes tool names. `ToolGateExecutor` is the last line
//! of defense: allow-list + deny-list, no real side effects, allow/deny written
//! to `audit.jsonl`.
//!
//! This is **not** a full LLM proxy. It proves the gate + audit shape that a
//! later API adapter can sit behind.
//!
//! ```bash
//! cargo run --example tool_gate
//! ```

use omega_cognitive_kernel::{
    AuditLog, AuditSummary, Executor, Result, ToolGateExecutor,
};
use omega_cognitive_kernel::types::Action;
use std::collections::HashMap;
use std::env::temp_dir;
use std::path::PathBuf;

fn tool(name: &str) -> Action {
    Action {
        name: name.into(),
        params: HashMap::new(),
        constraints_checked: true,
    }
}

fn main() -> Result<()> {
    let root: PathBuf = temp_dir().join(format!("omega-tool-gate-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).ok();
    let state_marker = root.join("state.json"); // audit lives beside this path
    let audit = AuditLog::beside_state(&state_marker);

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Omega tool-gate — block high-risk tool calls    ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!("audit: {}", audit.path.display());
    println!();

    // Safe tools an agent may call; dangerous names explicitly denied.
    let mut gate = ToolGateExecutor::new([
        "search_docs".into(),
        "http_get".into(),
        "summarize".into(),
    ])
    .deny([
        "delete_database".into(),
        "shell_exec".into(),
        "send_wire_transfer".into(),
    ])
    .with_audit(AuditLog {
        path: audit.path.clone(),
    })
    .echo(true)
    .fail_closed(false); // continue batch after deny so demo shows mixed outcomes

    // Simulated model proposals (not from DecisionMaker — host/agent layer).
    let proposals = [
        tool("search_docs"),
        tool("http_get"),
        tool("delete_database"), // should DENY
        tool("summarize"),
        tool("shell_exec"),      // should DENY
        tool("unknown_tool"),    // not on allow-list → DENY
    ];

    println!("── model proposed tools ──");
    let results = gate.execute(&proposals)?;
    for r in &results {
        println!("  → {r}");
    }

    let sum: AuditSummary = AuditLog {
        path: audit.path.clone(),
    }
    .summarize()?;
    println!("\n── audit summary ──");
    println!("events={}  tools_allowed={}  tools_denied={}", sum.total_events, sum.tools_allowed, sum.tools_denied);
    for (name, n) in &sum.action_counts {
        println!("  {name:<24} {n}");
    }

    println!("\n── takeaway ──");
    println!("1. Proposals come from outside (fake LLM / agent).");
    println!("2. ToolGateExecutor allow + deny is the enforcement point.");
    println!("3. Denies are auditable (ToolDenied) — no side effects executed.");
    println!("4. Next engineering step: wire real HTTP tools only on ALLOW path.");

    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}
