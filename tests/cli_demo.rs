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
    let binary = omega_kernel_binary();
    let state_arg = state.to_str().expect("UTF-8 state path");

    let demo = Command::new(&binary)
        .args(["--state", state_arg, "demo", "--steps", "3"])
        .output()
        .expect("run demo");
    assert!(
        demo.status.success(),
        "demo failed: {}",
        String::from_utf8_lossy(&demo.stderr)
    );
    assert!(state.is_file(), "demo did not save the requested state");

    let status = Command::new(&binary)
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

    let audit = Command::new(&binary)
        .args(["--state", state_arg, "audit", "--summary"])
        .output()
        .expect("run audit");
    assert!(audit.status.success());
    let audit = String::from_utf8(audit.stdout).expect("audit UTF-8");
    assert!(audit.contains("steps              : 3"), "{audit}");

    let _ = std::fs::remove_dir_all(dir);
}

fn omega_kernel_binary() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("CARGO_BIN_EXE_omega-kernel") {
        return path.into();
    }

    // Older Cargo versions may not expose this variable to integration tests.
    // The harness lives under <target-profile>/deps; the CLI is under <target-profile>.
    let test_executable =
        std::env::current_exe().expect("integration test executable should have a path");
    let profile_dir = test_executable
        .parent()
        .and_then(std::path::Path::parent)
        .expect("integration test executable should live in <target-profile>/deps");
    let binary = profile_dir.join(format!("omega-kernel{}", std::env::consts::EXE_SUFFIX));
    assert!(
        binary.is_file(),
        "Cargo did not set CARGO_BIN_EXE_omega-kernel and the CLI binary was not found at {}",
        binary.display()
    );
    binary
}

#[test]
#[cfg(feature = "toml")]
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

    let binary = omega_kernel_binary();
    let status = Command::new(&binary)
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

    let status = Command::new(&binary)
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
    let binary = omega_kernel_binary();
    let run = |extra: &[&str]| {
        let output = Command::new(&binary)
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

    let rejected = Command::new(&binary)
        .args(["benchmark-l2", "--no-purity-prior", "--json"])
        .output()
        .expect("reject orphan ablation");
    assert!(!rejected.status.success());
}
#[test]
fn cli_checkpoint_and_rollback_restores_prior_state() {
    let dir = std::env::temp_dir().join(format!(
        "omega-cli-rollback-test-{}-{}",
        std::process::id(),
        omega_cognitive_kernel::new_uuid()
    ));
    std::fs::create_dir_all(&dir).expect("create test directory");
    let state = dir.join("state.json");
    let binary = omega_kernel_binary();
    let state_arg = state.to_str().expect("UTF-8 state path");

    let run1 = Command::new(&binary)
        .args(["--state", state_arg, "run", "--steps", "3"])
        .output()
        .expect("run initial steps");
    assert!(
        run1.status.success(),
        "initial run failed: {}",
        String::from_utf8_lossy(&run1.stderr)
    );

    let cp = Command::new(&binary)
        .args([
            "--state",
            state_arg,
            "checkpoint",
            "--label",
            "pre-mutation",
        ])
        .output()
        .expect("create checkpoint");
    assert!(
        cp.status.success(),
        "checkpoint failed: {}",
        String::from_utf8_lossy(&cp.stderr)
    );
    let cp_stdout = String::from_utf8(cp.stdout).expect("checkpoint UTF-8");
    assert!(cp_stdout.contains("(step=3)"), "{cp_stdout}");
    let cp_id = cp_stdout
        .split("checkpoint created:")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("checkpoint uuid in stdout")
        .to_string();

    let list = Command::new(&binary)
        .args(["--state", state_arg, "checkpoints"])
        .output()
        .expect("list checkpoints");
    assert!(list.status.success());
    let list_stdout = String::from_utf8(list.stdout).expect("checkpoints UTF-8");
    assert!(list_stdout.contains(&cp_id), "{list_stdout}");
    assert!(list_stdout.contains("pre-mutation"), "{list_stdout}");

    let run2 = Command::new(&binary)
        .args(["--state", state_arg, "run", "--steps", "4"])
        .output()
        .expect("run additional steps");
    assert!(
        run2.status.success(),
        "second run failed: {}",
        String::from_utf8_lossy(&run2.stderr)
    );

    let mid_status = Command::new(&binary)
        .args(["--state", state_arg, "status", "--json"])
        .output()
        .expect("read mid status");
    assert!(mid_status.status.success());
    let mid_json: serde_json::Value =
        serde_json::from_slice(&mid_status.stdout).expect("mid status JSON");
    assert_eq!(mid_json["total_steps"], 7);

    let rb = Command::new(&binary)
        .args(["--state", state_arg, "rollback", &cp_id])
        .output()
        .expect("run rollback");
    assert!(
        rb.status.success(),
        "rollback failed: {}",
        String::from_utf8_lossy(&rb.stderr)
    );
    let rb_stdout = String::from_utf8(rb.stdout).expect("rollback UTF-8");
    assert!(rb_stdout.contains("rolled back to"), "{rb_stdout}");
    assert!(rb_stdout.contains("now at step 3"), "{rb_stdout}");

    let after_status = Command::new(&binary)
        .args(["--state", state_arg, "status", "--json"])
        .output()
        .expect("read after-rollback status");
    assert!(after_status.status.success());
    let after_json: serde_json::Value =
        serde_json::from_slice(&after_status.stdout).expect("after status JSON");
    assert_eq!(after_json["total_steps"], 3);

    let _ = std::fs::remove_dir_all(dir);
}
