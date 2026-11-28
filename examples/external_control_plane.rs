//! Demonstrates using `ClosedLoopSession` with `LoopPreset::ControlPlane` as a
//! cross-process persistent external control plane with `AuditLog` replay.
//!
//! Run with:
//!     cargo run --example external_control_plane

use omega_cognitive_kernel::api::{
    ClosedLoopKernel, ClosedLoopSession, DefaultKernel, ExternalOutcome, StepTelemetry,
};
use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::config::{KernelConfig, LoopPreset};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("omega-ext-example-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let state_path = dir.join("state.json");
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::ControlPlane);

    {
        let mut kernel = DefaultKernel::with_state_path(state_path.clone());
        kernel.autosave = true;
        kernel.dump_trajectory = false;
        let mut session = ClosedLoopSession::from_kernel(kernel, &cfg);
        let d1 = session
            .step_external(StepTelemetry {
                step_id: 1,
                prediction_error: 0.60,
                observed_risk: 0.20,
                signal_window: vec![0.60],
                last_outcome: None,
            })
            .map_err(Box::<dyn std::error::Error>::from)?;
        println!(
            "Process A -> gate={:?} patch={:?}",
            d1.l1_gate, d1.rule_patch
        );
    }

    let state_str = state_path.to_str().ok_or("non-UTF8 state path")?;
    let mut reopened = DefaultKernel::open(Some(state_str))?;
    reopened.autosave = true;
    reopened.dump_trajectory = false;
    let mut session = ClosedLoopSession::from_kernel(reopened, &cfg);

    let d2 = session
        .step_external(StepTelemetry {
            step_id: 2,
            prediction_error: 0.65,
            observed_risk: 0.20,
            signal_window: vec![0.65],
            last_outcome: Some(ExternalOutcome::Degraded),
        })
        .map_err(Box::<dyn std::error::Error>::from)?;
    println!(
        "Process B -> gate={:?} rollback={:?} cooldown={}",
        d2.l1_gate,
        d2.rollback_to_checkpoint,
        session.cooldown().current_cooldown()
    );

    let summary = AuditLog::beside_state(&state_path).summarize()?;
    println!(
        "AuditLog  -> steps={} rollbacks={}",
        summary.steps, summary.rollbacks
    );

    let _ = std::fs::remove_dir_all(dir);
    Ok(())
}
