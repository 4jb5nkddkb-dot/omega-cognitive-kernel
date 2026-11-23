use std::process::Command;

#[test]
fn demo_honors_explicit_state_for_status_and_audit() {
    let dir = std::env::temp_dir().join(format!(
        "omega-cli-demo-test-{}",
        omega_cognitive_kernel::new_uuid()
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
    assert_eq!(status["schema_version"], 5);
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
