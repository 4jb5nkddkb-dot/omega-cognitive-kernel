//! Operator audit panel: summarize audit.jsonl after a short run.
//!
//! ```bash
//! cargo run --example audit_panel
//! # or against an existing state file:
//! cargo run --example audit_panel -- ./path/to/state.json
//! ```

use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::{Kernel, Result};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let arg = env::args().nth(1);
    let state = match arg {
        Some(p) => PathBuf::from(p),
        None => {
            let path =
                env::temp_dir().join(format!("omega-audit-panel-{}.json", uuid::Uuid::new_v4()));
            let mut k = Kernel::with_state_path(&path);
            k.autosave = true;
            k.auto_checkpoint_on_patch = true;
            let _ = k.run("generate auditable steps for panel demo", 8)?;
            k.save()?;
            println!("(generated demo state at {})\n", path.display());
            path
        }
    };

    let log = AuditLog::beside_state(&state);
    let s = log.summarize()?;

    println!("╔══════════════════════════════════════════════╗");
    println!("║           Omega audit panel                  ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("state      : {}", state.display());
    println!("audit log  : {}", log.path.display());
    println!("events     : {}", s.total_events);
    println!(
        "steps={}  patches_ok={}  patches_rej={}  ckpts={}  rollbacks={}",
        s.steps, s.patches_accepted, s.patches_rejected, s.checkpoints, s.rollbacks
    );
    if !s.action_counts.is_empty() {
        println!("── action histogram ──");
        for (name, n) in &s.action_counts {
            println!("  {name:<12} {n}");
        }
    }
    if let Some(a) = &s.last_action {
        println!(
            "last: action={a} err={} damp={}",
            s.last_avg_err
                .map(|e| format!("{e:.3}"))
                .unwrap_or_else(|| "-".into()),
            s.last_damping
                .map(|d| format!("{d:.3}"))
                .unwrap_or_else(|| "-".into()),
        );
    }

    println!("\nCLI equivalent:");
    println!("  omega-kernel --state {} audit --summary", state.display());
    Ok(())
}
