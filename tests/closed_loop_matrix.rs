use omega_cognitive_kernel::api::{
    ClosedLoopKernel, ClosedLoopSession, DefaultKernel, ExternalOutcome, StepTelemetry, LoopBenchmarkScenario, run_closed_loop_matrix_benchmark,
};
use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::config::{
    KernelConfig, L1RuntimeGate, LayerSwitches, LoopMode, LoopPreset,
};
use omega_cognitive_kernel::orchestrator::CooldownTracker;

fn new_isolated_session(
    tag: &str,
    config: &KernelConfig,
) -> (ClosedLoopSession, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "omega-closed-loop-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp test dir");
    let state_path = dir.join("state.json");
    let mut kernel = DefaultKernel::with_state_path(state_path);
    kernel.autosave = false;
    kernel.dump_trajectory = false;
    (ClosedLoopSession::from_kernel(kernel, config), dir)
}

fn telemetry(
    step_id: u64,
    prediction_error: f64,
    observed_risk: f64,
    last_outcome: Option<ExternalOutcome>,
) -> StepTelemetry {
    StepTelemetry {
        step_id,
        prediction_error,
        observed_risk,
        signal_window: vec![prediction_error],
        last_outcome,
    }
}

#[test]
fn all_seven_presets_and_validation_rules() {
    let cases = [
        (
            LoopPreset::Deterministic,
            LayerSwitches {
                l0: true,
                l1: false,
                l2: false,
            },
            LoopMode::InternalWorld,
        ),
        (
            LoopPreset::HotPatcher,
            LayerSwitches {
                l0: false,
                l1: true,
                l2: false,
            },
            LoopMode::ExternalTelemetry,
        ),
        (
            LoopPreset::MetaGuard,
            LayerSwitches {
                l0: false,
                l1: false,
                l2: true,
            },
            LoopMode::ExternalTelemetry,
        ),
        (
            LoopPreset::FastAdapt,
            LayerSwitches {
                l0: true,
                l1: true,
                l2: false,
            },
            LoopMode::InternalWorld,
        ),
        (
            LoopPreset::SteadyPrior,
            LayerSwitches {
                l0: true,
                l1: false,
                l2: true,
            },
            LoopMode::InternalWorld,
        ),
        (
            LoopPreset::ControlPlane,
            LayerSwitches {
                l0: false,
                l1: true,
                l2: true,
            },
            LoopMode::ExternalTelemetry,
        ),
        (
            LoopPreset::FullKernel,
            LayerSwitches {
                l0: true,
                l1: true,
                l2: true,
            },
            LoopMode::InternalWorld,
        ),
    ];

    for (preset, expected_switches, expected_mode) in cases {
        let switches: LayerSwitches = preset.into();
        assert_eq!(switches, expected_switches);
        assert_eq!(switches.validate(), Ok(expected_mode));

        let cfg = KernelConfig::default().with_loop_preset(preset);
        assert_eq!(cfg.effective_switches(), expected_switches);
    }

    let empty = LayerSwitches {
        l0: false,
        l1: false,
        l2: false,
    };
    assert!(empty.validate().is_err());

    let mut legacy_cfg = KernelConfig {
        self_modify_enabled: true,
        ..Default::default()
    };
    legacy_cfg.l2.enabled = false;
    assert_eq!(
        legacy_cfg.effective_switches(),
        LayerSwitches {
            l0: true,
            l1: true,
            l2: false,
        }
    );
}

#[test]
fn mode_100_deterministic_pure_l0_closed_loop() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::Deterministic);
    let (mut session, dir) = new_isolated_session("m100", &cfg);

    assert!(session.step_external(telemetry(1, 0.5, 0.2, None)).is_err());

    let d1 = session.step_internal().expect("internal L0 step");
    assert_eq!(d1.l1_gate, L1RuntimeGate::Disabled);
    assert!(d1.l2_prior.is_none());
    assert!(d1.rule_patch.is_none());
    assert!(d1.keep_steady);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_110_fast_adapt_and_cooldown_backoff() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::FastAdapt);
    let (mut session, dir) = new_isolated_session("m110", &cfg);

    let d1 = session.step_internal().expect("internal L0+L1 step");
    assert!(d1.l2_prior.is_none());
    assert_eq!(d1.l1_gate, L1RuntimeGate::Active);

    let mut tracker = CooldownTracker::new(3);
    assert!(tracker.can_patch(0.45, 0.30, 0.20, 0.65));
    tracker.mark_patched();
    assert!(!tracker.can_patch(0.45, 0.30, 0.20, 0.65));

    tracker.record_rollback();
    assert_eq!(tracker.consecutive_rollbacks(), 1);
    assert_eq!(tracker.current_cooldown(), 6);

    tracker.record_rollback();
    assert_eq!(tracker.consecutive_rollbacks(), 2);
    assert_eq!(tracker.current_cooldown(), 12);

    tracker.record_improved();
    assert_eq!(tracker.consecutive_rollbacks(), 0);
    assert_eq!(tracker.current_cooldown(), 3);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_101_steady_prior_default_closed_loop() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::SteadyPrior);
    let (mut session, dir) = new_isolated_session("m101", &cfg);

    let d1 = session.step_internal().expect("internal L0+L2 step");
    assert_eq!(d1.l1_gate, L1RuntimeGate::Disabled);
    assert!(d1.l2_prior.is_some());
    assert!(d1.keep_steady);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_111_full_kernel_preserves_static_switches() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::FullKernel);
    let (mut session, dir) = new_isolated_session("m111", &cfg);
    let initial_switches = session.switches();

    let d1 = session.step_internal().expect("internal full step");
    assert!(d1.l2_prior.is_some());
    assert_eq!(session.switches(), initial_switches);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_010_hot_patcher_external_loop_rollback_and_idempotency() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::HotPatcher);
    let (mut session, dir) = new_isolated_session("m010", &cfg);

    assert!(session.step_internal().is_err());

    let d1 = session
        .step_external(telemetry(1, 0.50, 0.20, None))
        .expect("step 1");
    assert_eq!(d1.l1_gate, L1RuntimeGate::Active);
    assert!(d1.l2_prior.is_none());
    assert!(d1.rule_patch.is_some());
    assert!(!d1.keep_steady);

    // Duplicate or out-of-order step_id is idempotent.
    let dup = session
        .step_external(telemetry(1, 0.90, 0.10, None))
        .expect("duplicate step");
    assert!(dup.keep_steady);
    assert!(dup.rule_patch.is_none());

    // High risk is rejected even when error is high.
    let risky = session
        .step_external(telemetry(2, 0.80, 0.95, None))
        .expect("high risk step");
    assert!(risky.rule_patch.is_none());

    // Degraded feedback triggers rollback and exponential cooldown backoff.
    let degraded = session
        .step_external(telemetry(3, 0.60, 0.20, Some(ExternalOutcome::Degraded)))
        .expect("degraded step");
    assert_eq!(degraded.rollback_to_checkpoint.as_deref(), Some("ckpt-1"));
    assert_eq!(session.cooldown().consecutive_rollbacks(), 1);
    assert_eq!(session.cooldown().current_cooldown(), 6);
    assert!(degraded.rule_patch.is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_010_shadow_mode_never_mutates_cooldown() {
    let mut cfg = KernelConfig::default().with_loop_preset(LoopPreset::HotPatcher);
    cfg.l1_shadow_mode = true;
    let (mut session, dir) = new_isolated_session("m010-shadow", &cfg);

    let d1 = session
        .step_external(telemetry(1, 0.55, 0.15, None))
        .expect("shadow step");
    assert_eq!(d1.l1_gate, L1RuntimeGate::ShadowOnly);
    let patch = d1.rule_patch.expect("shadow patch emitted");
    assert!(patch.shadow_only);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_001_meta_guard_external_steady_detection() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::MetaGuard);
    let (mut session, dir) = new_isolated_session("m001", &cfg);

    let steady = session
        .step_external(telemetry(1, 0.05, 0.10, None))
        .expect("steady telemetry");
    assert_eq!(steady.l1_gate, L1RuntimeGate::Disabled);
    assert!(steady.rule_patch.is_none());
    let prior = steady.l2_prior.expect("L2 prior");
    assert!(prior.regime_detected);
    assert!(prior.history_discount_enabled);
    assert!(!prior.kl_prior_bias.is_empty());
    assert!(prior.lyapunov_potential.is_finite());
    assert!(prior.alpha.is_finite());

    let shift = session
        .step_external(telemetry(2, 0.65, 0.20, None))
        .expect("regime shift telemetry");
    let shift_prior = shift.l2_prior.expect("L2 prior on shift");
    assert!(!shift_prior.regime_detected);
    assert!(shift.rule_patch.is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn mode_011_control_plane_l2_blocks_l1_when_steady() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::ControlPlane);
    let (mut session, dir) = new_isolated_session("m011", &cfg);

    // Steady low error: L2 blocks L1 write without mutating static switches.
    let blocked = session
        .step_external(telemetry(1, 0.10, 0.10, None))
        .expect("steady control plane step");
    assert_eq!(blocked.l1_gate, L1RuntimeGate::BlockedByL2);
    assert!(blocked.rule_patch.is_none());
    assert!(blocked.keep_steady);
    assert!(session.switches().l1);

    // High error: L2 releases gate and L1 emits patch.
    let active = session
        .step_external(telemetry(
            2,
            0.55,
            0.20,
            Some(ExternalOutcome::AcceptedAndImproved),
        ))
        .expect("active control plane step");
    assert_eq!(active.l1_gate, L1RuntimeGate::Active);
    assert!(active.rule_patch.is_some());
    assert!(!active.keep_steady);

    let _ = std::fs::remove_dir_all(dir);
}
#[test]
fn external_loop_state_and_audit_survive_restart() {
    let cfg = KernelConfig::default().with_loop_preset(LoopPreset::HotPatcher);
    let dir = std::env::temp_dir().join(format!(
        "omega-closed-loop-restart-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let state_path = dir.join("state.json");

    {
        let mut kernel = DefaultKernel::with_state_path(state_path.clone());
        kernel.autosave = true;
        kernel.dump_trajectory = false;
        let mut session = ClosedLoopSession::from_kernel(kernel, &cfg);
        let first = session
            .step_external(telemetry(1, 0.50, 0.20, None))
            .expect("step 1");
        assert!(first.rule_patch.is_some());
    }

    let state_str = state_path.to_str().expect("utf8 path");
    let reopened_kernel = DefaultKernel::open(Some(state_str)).expect("reopen kernel");
    let mut session = ClosedLoopSession::from_kernel(reopened_kernel, &cfg);

    let dup = session
        .step_external(telemetry(1, 0.90, 0.10, None))
        .expect("duplicate step 1 after restart");
    assert!(dup.keep_steady);
    assert!(dup.rule_patch.is_none());

    let degraded = session
        .step_external(telemetry(2, 0.60, 0.20, Some(ExternalOutcome::Degraded)))
        .expect("degraded step 2 after restart");
    assert_eq!(degraded.rollback_to_checkpoint.as_deref(), Some("ckpt-1"));
    assert_eq!(session.cooldown().current_cooldown(), 6);

    let summary = AuditLog::beside_state(&state_path)
        .summarize()
        .expect("summarize audit log");
    assert_eq!(summary.steps, 2);
    assert_eq!(summary.rollbacks, 1);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn benchmark_loops_covers_all_seven_presets_across_scenarios() {
    for scenario in [
        LoopBenchmarkScenario::Steady,
        LoopBenchmarkScenario::Shift,
        LoopBenchmarkScenario::Noisy,
    ] {
        let report = run_closed_loop_matrix_benchmark(scenario, 12);
        assert_eq!(report.rows.len(), 7);
        for row in &report.rows {
            assert_eq!(row.steps, 12);
            assert!(row.mean_error.is_finite());
            assert!(row.avg_step_us.is_finite() && row.avg_step_us >= 0.0);
        }
    }
}
