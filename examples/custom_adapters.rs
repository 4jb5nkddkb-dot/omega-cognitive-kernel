//! Example: inject custom Observer + Executor into the kernel.
//!
//! Run with:
//! ```bash
//! cargo run --example custom_adapters
//! ```

use omega_cognitive_kernel::{
    FileObserver, Kernel, LoggingExecutor, Result,
};
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<()> {
    // Prepare a tiny status file the FileObserver will read.
    let status_path = PathBuf::from("/tmp/omega-status-example.json");
    {
        let mut f = std::fs::File::create(&status_path).expect("create status file");
        writeln!(
            f,
            r#"{{"signal": "external", "load": 0.42, "note": "hello from file"}}"#
        )
        .expect("write status");
    }

    let log_path = PathBuf::from("/tmp/omega-actions.log");
    let _ = std::fs::remove_file(&log_path);

    let observer = FileObserver::new(&status_path);
    let executor = LoggingExecutor::new([
        "noop".into(),
        "observe".into(),
        "explore".into(),
        "exploit".into(),
        "stabilize".into(),
    ])
    .with_log_file(&log_path)
    .echo_stdout(true);

    let mut kernel = Kernel::with_adapters(observer, executor);
    kernel.autosave = false;
    kernel.auto_checkpoint_on_patch = false;

    let report = kernel.run("adapt to external file signal", 5)?;
    println!(
        "\nReport: steps={} err {:.3} → {:.3} decreased={} actions={:?}",
        report.steps,
        report.early_avg_err,
        report.final_avg_err,
        report.error_decreased,
        report.last_actions
    );

    println!("\nAction log written to {}", log_path.display());
    if let Ok(log) = std::fs::read_to_string(&log_path) {
        println!("{log}");
    }

    Ok(())
}
