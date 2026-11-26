use std::process::Command;

#[test]
fn demo_honors_explicit_state_for_status_and_audit() {
    let dir = std::env::temp_dir().join(format!(
        "omega-cli-demo-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create test directory");
    let state = dir.join("state.json");
    let binary = env!("CARGO_BIN_EXE_omega-kernel");
    let state_arg = state.to_str().expect("UTF-8 state path");

    let demo = Command::new(binary)
        .args(["--state", state_arg, "demo", "--steps", "3"])
        .output()
        .expect("run demo");
    assert!(
        demo.status.success(),
        "demo failed: {}",
        String::from_utf8_lossy(&demo.stderr)
    );
    assert!(state.is_file(), "demo did not save the requested state");

    let status = Command::new(binary)
        .args(["--state", state_arg, "status", "--json"])
        .output()
        .expect("run status");
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).expect("status JSON");
    assert_eq!(status["total_steps"], 3);
    assert_eq!(
        status["schema_version"],
        omega_cognitive_kernel::persist::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(status["l2_gate_state"], "DORMANT");
    assert!(status["last_l2_decision"].is_null());

    let audit = Command::new(binary)
        .args(["--state", state_arg, "audit", "--summary"])
        .output()
        .expect("run audit");
    assert!(audit.status.success());
    let audit = String::from_utf8(audit.stdout).expect("audit UTF-8");
    assert!(audit.contains("steps              : 3"), "{audit}");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn config_selects_and_loads_the_configured_state_before_override() {
    let dir = std::env::temp_dir().join(format!(
        "omega-cli-config-test-{}-{}",
        std::process::id(),
        omega_cognitive_kernel::new_uuid()
    ));
    std::fs::create_dir_all(&dir).expect("create test directory");
    let state = dir.join("configured.json");
    let override_state = dir.join("override.json");
    let config = dir.join("omega.toml");
    std::fs::write(
        &config,
        format!(
            "state_path = {:?}\nautosave = false\ndump_trajectory = false\n",
            state.to_string_lossy()
        ),
    )
    .expect("write config");

    let binary = env!("CARGO_BIN_EXE_omega-kernel");
    let status = Command::new(binary)
        .args(["--config", config.to_str().unwrap(), "run", "--steps", "2"])
        .output()
        .expect("run with configured state");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    assert_eq!(snapshot["total_steps"], 2);

    let status = Command::new(binary)
        .args([
            "--config",
            config.to_str().unwrap(),
            "--state",
            override_state.to_str().unwrap(),
            "run",
            "--steps",
            "3",
        ])
        .output()
        .expect("run with state override");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let snapshot: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&override_state).unwrap()).unwrap();
    assert_eq!(snapshot["total_steps"], 3);
    let original: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    assert_eq!(original["total_steps"], 2);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn benchmark_spectral_intervention_is_explicitly_opt_in() {
    let binary = env!("CARGO_BIN_EXE_omega-kernel");
    let run = |extra: &[&str]| {
        let output = Command::new(binary)
            .args([
                "benchmark-l2",
                "--scenario",
                "regime_shift",
                "--runs",
                "1",
                "--episodes",
                "2",
                "--steps",
                "4",
                "--seed",
                "20261115",
                "--json",
            ])
            .args(extra)
            .output()
            .expect("run benchmark");
        assert!(
            output.status.success(),
            "benchmark failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).expect("benchmark JSON")
    };

    let default = run(&[]);
    assert!(default["paired_runs"][0]["treatment"]["experiment"].is_null());
    let diagnostics = run(&["--diagnostics"]);
    assert_eq!(
        diagnostics["paired_runs"][0]["treatment"]["experiment"]["spectral_control"],
        false
    );
    let treatment = run(&["--spectral-control"]);
    assert_eq!(
        treatment["paired_runs"][0]["treatment"]["experiment"]["spectral_control"],
        true
    );

    let rejected = Command::new(binary)
        .args(["benchmark-l2", "--no-purity-prior", "--json"])
        .output()
        .expect("reject orphan ablation");
    assert!(!rejected.status.success());
}
