//! Event stream demo: in-process hook + file-tail poll.
//!
//! Shows the two integrator paths added in 0.5.2+:
//! 1. `Kernel::set_event_hook` — synchronous callback after each audit append
//! 2. `AuditLog::poll_after` / `byte_len` — out-of-process or multi-reader tail
//!
//! ```bash
//! cargo run --example event_hook_poll
//! ```

use omega_cognitive_kernel::{
    AuditEvent, AuditLog, Kernel, KernelEvent, Result,
};
use std::env::temp_dir;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let root = temp_dir().join(format!("omega-events-{}", uuid::Uuid::new_v4()));
    let state_path = root.join("state.json");
    std::fs::create_dir_all(&root).expect("mkdir");

    println!("=== Omega event stream demo ===");
    println!("state : {}", state_path.display());
    println!("audit : {}", root.join("audit.jsonl").display());
    println!();

    // ── 1. In-process hook ──────────────────────────────────────────────
    let hook_hits = Arc::new(AtomicUsize::new(0));
    let hits_for_hook = Arc::clone(&hook_hits);

    let mut kernel = Kernel::with_state_path(&state_path);
    kernel.set_event_hook(move |ev: &AuditEvent| {
        // KernelEvent is a type alias of AuditEvent — either name works.
        let kind = match ev {
            AuditEvent::Step { action, .. } => format!("step:{action}"),
            AuditEvent::PatchAccepted { .. } => "patch_accepted".into(),
            AuditEvent::PatchRejected { reason, .. } => format!("patch_rejected:{reason}"),
            AuditEvent::Checkpoint { label, .. } => format!("checkpoint:{label}"),
            other => format!("{other:?}"),
        };
        hits_for_hook.fetch_add(1, Ordering::Relaxed);
        println!("[hook] {kind}");
    });

    // ── 2. File-tail cursor (can live in another process) ───────────────
    let log = AuditLog::beside_state(&state_path);
    let mut cursor = log.byte_len()?;

    let goal = "exercise event stream under a short run";
    for i in 1..=6u32 {
        let _ = kernel.step(goal)?;

        // Poll only bytes written since last cursor (no full re-read).
        let (rows, next) = log.poll_after(cursor)?;
        for rec in &rows {
            // Same shape as the hook payload.
            let _ev: &KernelEvent = &rec.event;
            println!(
                "[poll] ts={} kind={:?}",
                rec.ts.to_rfc3339(),
                event_kind(&rec.event)
            );
        }
        cursor = next;

        if i == 3 {
            // Operator-style weight nudge also emits nothing by itself;
            // it only affects subsequent decisions. Shown for API parity.
            let _ = kernel.set_strategy_weight("explore", 1.2);
        }

        thread::sleep(Duration::from_millis(20));
    }

    kernel.clear_event_hook();
    kernel.save()?;

    println!("\n── summary ──");
    println!("hook invocations : {}", hook_hits.load(Ordering::Relaxed));
    println!("audit file bytes : {}", log.byte_len()?);
    let summary = log.summarize()?;
    println!(
        "events={} steps={} patches_accepted={}",
        summary.total_events, summary.steps, summary.patches_accepted
    );
    println!("\nCleanup: remove {}", root.display());
    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}

fn event_kind(ev: &AuditEvent) -> &'static str {
    match ev {
        AuditEvent::Step { .. } => "step",
        AuditEvent::PatchAccepted { .. } => "patch_accepted",
        AuditEvent::PatchRejected { .. } => "patch_rejected",
        AuditEvent::Checkpoint { .. } => "checkpoint",
        AuditEvent::Rollback { .. } => "rollback",
        AuditEvent::TaskEpisode { .. } => "task_episode",
        AuditEvent::ToolAllowed { .. } => "tool_allowed",
        AuditEvent::ToolDenied { .. } => "tool_denied",
    }
}
