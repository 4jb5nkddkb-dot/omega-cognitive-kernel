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
            let dir = env::temp_dir().join(format!(
                "omega-audit-panel-{}-{}",
                std::process::id(),
                omega_cognitive_kernel::new_uuid()
            ));
            std::fs::create_dir_all(&dir)
                .map_err(|error| omega_cognitive_kernel::OmegaError::Persist(error.to_string()))?;
            let path = dir.join("state.json");
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

#[cfg(test)]
mod tests {
    use omega_cognitive_kernel::audit::{AuditEvent, AuditLog, AuditRecord, AuditSummary};
    use omega_cognitive_kernel::Kernel;
    use std::env::temp_dir;

    #[test]
    fn audit_panel_summarizes_demo_state() {
        // Create a temporary state file with some demo data
        let dir = temp_dir().join(format!(
            "omega-audit-test-{}-{}",
            std::process::id(),
            omega_cognitive_kernel::new_uuid()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");

        // Create kernel and run some steps
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        let _ = k.run("test audit panel", 5);
        k.save().unwrap();

        // Test audit log summarization
        let log = AuditLog::beside_state(&path);
        let summary = log.summarize().unwrap();

        assert!(summary.total_events > 0);
        assert!(summary.steps > 0);
        assert_eq!(summary.patches_accepted, 0); // self_modify_enabled = false by default
        assert!(summary.patches_rejected > 0);

        // Test that we can open the kernel and get status
        let kernel = Kernel::open(Some(path.to_str().unwrap())).unwrap();
        let status = kernel.status();
        assert!(!status.self_modify_enabled);
        assert!(status.strategy_count > 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn audit_panel_handles_missing_state() {
        // Use a unique temp subdirectory to avoid interference from other tests
        // that might create audit logs in the same temp directory.
        let unique_dir = temp_dir().join(format!(
            "omega-audit-test-{}-{}",
            std::process::id(),
            omega_cognitive_kernel::new_uuid()
        ));
        std::fs::create_dir_all(&unique_dir).unwrap();
        let path = unique_dir.join("nonexistent_state.json");
        let log = AuditLog::beside_state(&path);
        let summary = log.summarize().unwrap();

        assert_eq!(summary.total_events, 0);
        assert_eq!(summary.steps, 0);
    }

    #[test]
    fn audit_summary_from_records() {
        let now = chrono::Utc::now();
        let rows = vec![
            AuditRecord {
                ts: now,
                event: AuditEvent::Step {
                    step: 1,
                    goal: "test".into(),
                    action: "stabilize".into(),
                    avg_err: 0.5,
                    damping: 1.0,
                },
            },
            AuditRecord {
                ts: now,
                event: AuditEvent::PatchRejected {
                    step: 1,
                    reason: "self_modify_enabled=false".into(),
                },
            },
            AuditRecord {
                ts: now,
                event: AuditEvent::Step {
                    step: 2,
                    goal: "test".into(),
                    action: "explore".into(),
                    avg_err: 0.3,
                    damping: 0.9,
                },
            },
        ];

        let summary = AuditSummary::from_records(&rows);
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.steps, 2);
        assert_eq!(summary.patches_rejected, 1);
        assert_eq!(summary.action_counts.len(), 2);
        assert_eq!(summary.last_action, Some("explore".into()));
        assert!((summary.last_avg_err.unwrap() - 0.3).abs() < f64::EPSILON);
        assert!((summary.last_damping.unwrap() - 0.9).abs() < f64::EPSILON);
    }
}
