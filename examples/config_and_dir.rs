//! Config file + directory snapshot adapter (offline-friendly).
//!
//! ```bash
//! cargo run --example config_and_dir
//! ```

use omega_cognitive_kernel::{
    DirSnapshotObserver, Kernel, KernelConfig, LoggingExecutor, Result,
};
use std::env::temp_dir;

fn main() -> Result<()> {
    let work = temp_dir().join(format!("omega-phase1-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&work).expect("mkdir");
    std::fs::write(work.join("signal.txt"), b"load=0.4").expect("write signal");

    let cfg_path = work.join("omega.toml");
    let state_path = work.join("state.json");
    let mut cfg = KernelConfig::default();
    cfg.state_path = Some(state_path.display().to_string());
    cfg.autosave = false;
    cfg.auto_checkpoint_on_patch = false;
    cfg.max_risk = 0.5;
    cfg.error_threshold = 0.28;
    cfg.save(&cfg_path)?;

    println!("config: {}", cfg_path.display());
    println!("watching dir: {}", work.display());

    let observer = DirSnapshotObserver::new(&work).max_entries(32);
    let executor = LoggingExecutor::new([
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ])
    .echo_stdout(true);

    let mut kernel = Kernel::with_adapters_and_path(observer, executor, &state_path);
    kernel.apply_config(&KernelConfig::load(&cfg_path)?);

    let report = kernel.run("monitor drop directory and stabilize under uncertainty", 4)?;
    println!(
        "\nsteps={} err {:.3}→{:.3} decreased={} actions={:?}",
        report.steps,
        report.early_avg_err,
        report.final_avg_err,
        report.error_decreased,
        report.last_actions
    );
    println!("max_risk now {}", kernel.status().max_risk);

    let _ = std::fs::remove_dir_all(&work);
    Ok(())
}
