//! Omega Audit Panel — standalone operator product.
//!
//! Quick view of audit trail, patch counts, action histogram, and last step.
//!
//! ```bash
//! cargo run --release --bin omega-audit-panel
//! cargo run --release --bin omega-audit-panel -- ./path/to/state.json
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
            // Keep self-modify off for the demo product path.
            k.inner.l1.self_modify_enabled = false;
            let _ = k.run("generate auditable steps for panel demo", 10)?;
            k.save()?;
            println!("(demo state generated at {})\n", path.display());
            path
        }
    };

    let log = AuditLog::beside_state(&state);
    let s = log.summarize()?;

    println!("╔══════════════════════════════════════════════╗");
    println!("║         Omega Audit Panel  ·  product        ║");
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

    // Also show kernel status if state is loadable
    if let Ok(k) = Kernel::open(Some(state.to_str().unwrap_or(""))) {
        let st = k.status();
        println!("── kernel ──");
        println!(
            "self_modify={}  strategies={}  cases={}  in_fp={}",
            st.self_modify_enabled, st.strategy_count, st.case_count, st.in_fixed_point
        );
    }

    println!("\nCLI equivalent:");
    println!("  omega-kernel --state {} audit --summary", state.display());
    println!("  omega-kernel --state {} status", state.display());
    Ok(())
}
