//! Closed-loop product demo: watch a drop folder, adapt policy, log actions.
//!
//! Story for integrators:
//! 1. External system drops files into `inbox/`
//! 2. Kernel observes directory snapshots each step
//! 3. RuleEffect patches adjust explore/stabilize under rising "load"
//! 4. Actions are written to `actions.log` (audit-friendly side channel)
//!
//! ```bash
//! cargo run --example drop_inbox_loop
//! ```

use omega_cognitive_kernel::{
    DirSnapshotObserver, Kernel, KernelConfig, LoggingExecutor, Result,
};
use std::env::temp_dir;
use std::fs;
use std::thread;
use std::time::Duration;

fn main() -> Result<()> {
    let root = temp_dir().join(format!("omega-inbox-{}", uuid::Uuid::new_v4()));
    let inbox = root.join("inbox");
    let cfg_path = root.join("omega.toml");
    let state_path = root.join("state.json");
    let action_log = root.join("actions.log");

    fs::create_dir_all(&inbox).expect("mkdir inbox");

    let cfg = KernelConfig {
        state_path: Some(state_path.display().to_string()),
        autosave: true,
        auto_checkpoint_on_patch: true,
        error_threshold: 0.28,
        min_steps_between_patches: 2,
        max_risk: 0.55,
        default_goal: Some("stabilize under inbox load and keep audit trail".into()),
    };
    cfg.save(&cfg_path)?;

    println!("=== Omega drop-inbox loop ===");
    println!("root   : {}", root.display());
    println!("inbox  : {}", inbox.display());
    println!("config : {}", cfg_path.display());
    println!("actions: {}", action_log.display());
    println!();

    // Seed a few "arrivals" so the first observations are non-empty.
    fs::write(inbox.join("job-001.txt"), b"priority=low").ok();

    let observer = DirSnapshotObserver::new(&inbox).max_entries(128);
    let executor = LoggingExecutor::new([
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ])
    .with_log_file(&action_log)
    .echo_stdout(true);

    let mut kernel = Kernel::with_adapters_and_path(observer, executor, &state_path);
    kernel.apply_config(&KernelConfig::load(&cfg_path)?);

    let goal = cfg
        .default_goal
        .clone()
        .unwrap_or_else(|| "stabilize under inbox load".into());

    // Simulate an external producer while the kernel runs.
    for step in 1..=8u32 {
        // "World" side: more files → more load signal in dir snapshot.
        if step % 2 == 0 {
            let name = format!("job-{:03}.txt", step + 10);
            let body = format!("priority=high step={step}");
            let _ = fs::write(inbox.join(name), body);
            println!("[world] dropped file into inbox (step {step})");
        }

        let r = kernel.step(&goal)?;
        let act = r
            .trajectory
            .actions
            .first()
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let st = kernel.status();
        println!(
            "[kernel] step={step} act={act:<9} patched={} avg_err={:.3} damp={:.2} max_risk={:.2}",
            r.l1.patched, st.avg_prediction_error, st.error_damping, st.max_risk
        );

        thread::sleep(Duration::from_millis(30));
    }

    kernel.save()?;

    println!("\n── summary ──");
    let st = kernel.status();
    println!("steps={}  avg_err={:.3}  modules={}  ckpts={}", st.total_steps, st.avg_prediction_error, st.active_modules, st.checkpoint_count);
    if action_log.exists() {
        let log = fs::read_to_string(&action_log).unwrap_or_default();
        let lines = log.lines().count();
        println!("action log lines: {lines}  ({})", action_log.display());
    }
    println!("\nCleanup: remove {}", root.display());
    let _ = fs::remove_dir_all(&root);
    Ok(())
}
