"""Integration tests for the installed omega-cognitive-kernel Python wheel."""

import json
import math
from pathlib import Path

import pytest

import omega_cognitive_kernel_python as ok


@pytest.fixture
def state_path(tmp_path):
    return tmp_path / "state.json"


@pytest.fixture
def kernel(state_path):
    return ok.Kernel.with_state_path(str(state_path))


def test_initialization_and_persistence(state_path):
    kernel = ok.Kernel.with_state_path(str(state_path))
    assert Path(kernel.state_path()) == state_path
    assert kernel.status().total_steps == 0
    assert kernel.status().self_modify_enabled is False

    kernel.run("persist this state", 2)
    kernel.save()
    assert state_path.is_file()

    reopened = ok.Kernel.open(str(state_path))
    assert reopened.status().total_steps == 2
    assert Path(reopened.status().state_path) == state_path


def test_from_config_path_loads_toml(tmp_path):
    state_path = tmp_path / "configured-state.json"
    config_path = tmp_path / "kernel.toml"
    config_path.write_text(
        f"state_path = {json.dumps(str(state_path))}\nautosave = false\n",
        encoding="utf-8",
    )

    configured = ok.Kernel.from_config_path(str(config_path))
    assert Path(configured.state_path()) == state_path
    assert configured.status().total_steps == 0


def test_single_step_and_run(kernel):
    step = kernel.step("test goal")
    assert step.steps == 1
    assert step.patches_accepted == 0
    assert len(step.last_actions) == 1
    assert step.last_actions[0] in {"noop", "observe", "explore", "exploit", "stabilize"}
    assert math.isfinite(step.final_avg_err)

    report = kernel.run("another goal", 3)
    assert report.steps == 3
    assert report.patches_accepted == 0
    assert len(report.last_actions) == 3
    assert kernel.status().total_steps == 4


def test_checkpoints_and_rollback(kernel):
    kernel.run("initial goal", 5)
    checkpoint = kernel.create_checkpoint("before_changes")
    assert checkpoint.label == "before_changes"
    assert checkpoint.step == 5

    kernel.run("more goals", 3)
    assert kernel.status().total_steps == 8
    checkpoints = kernel.list_checkpoints()
    assert [item.id for item in checkpoints] == [checkpoint.id]

    kernel.rollback(checkpoint.id)
    assert kernel.status().total_steps == 5

    reopened = ok.Kernel.open(kernel.state_path())
    assert reopened.status().total_steps == 5


def test_snapshot_json_round_trip(kernel, state_path):
    kernel.run("snapshot goal", 2)
    snapshot = kernel.snapshot_json()
    payload = json.loads(snapshot)
    assert payload["total_steps"] == 2
    assert payload["version"] >= 1

    restored = ok.Kernel.with_state_path(str(state_path.with_name("restored.json")))
    restored.apply_snapshot_json(snapshot)
    assert restored.status().total_steps == kernel.status().total_steps
    assert restored.status().avg_prediction_error == pytest.approx(
        kernel.status().avg_prediction_error
    )

    with pytest.raises(ValueError, match="expected ident"):
        restored.apply_snapshot_json("not valid JSON")


def test_strategy_weight(kernel):
    report = kernel.run("create strategies", 1)
    strategy = report.last_actions[0]
    assert kernel.set_strategy_weight(strategy, 2.0) is True
    assert kernel.set_strategy_weight("unknown_strategy", 1.0) is False
    kernel.save()

    snapshot = json.loads(ok.Kernel.open(kernel.state_path()).snapshot_json())
    assert snapshot["strategies"][strategy]["weight"] == pytest.approx(2.0)


def test_l2_settings(kernel):
    # L2 is enabled by default.
    assert kernel.status().l2_enabled is True
    kernel.set_l2_enabled(True)
    assert kernel.status().l2_enabled is True
    kernel.set_l2_enabled(False)
    assert kernel.status().l2_enabled is False


def test_audit_size_limit_is_configurable_and_fails_closed(kernel, state_path):
    kernel.set_audit_max_bytes(0)

    with pytest.raises(Exception, match="audit log size limit exceeded"):
        kernel.step("this step cannot be audited")

    assert not (state_path.parent / "audit.jsonl").exists()


def test_trajectory_replay(kernel):
    kernel.set_dump_trajectory(True)
    kernel.run("replay test", 4)

    replay = kernel.replay_trajectory(3)
    assert Path(replay.path) == Path(kernel.state_path()).parent / "trajectory.jsonl"
    assert replay.rows == 4
    assert replay.patches == 0
    assert len(replay.tail) == 3
    assert [row.step for row in replay.tail] == [2, 3, 4]
    assert all(row.goal == "replay test" for row in replay.tail)
    assert sum(count for _, count in replay.action_counts) == 4


def test_trajectory_size_limit_setting(kernel):
    assert kernel.trajectory_max_bytes() == 64 * 1024 * 1024
    kernel.set_trajectory_max_bytes(4096)
    assert kernel.trajectory_max_bytes() == 4096


def test_trajectory_size_limit_fails_step_without_appending(kernel):
    kernel.set_trajectory_max_bytes(0)
    with pytest.raises(RuntimeError, match="trajectory size limit exceeded"):
        kernel.step("reject trajectory append")

    trajectory_path = Path(kernel.state_path()).parent / "trajectory.jsonl"
    assert not trajectory_path.exists()


def test_audit_log_tracks_steps_and_rollback(kernel):
    kernel.run("audit goal", 2)
    checkpoint = kernel.create_checkpoint("audit checkpoint")
    kernel.rollback(checkpoint.id)

    log = ok.AuditLog.beside_state(kernel.state_path())
    summary = log.summarize()
    assert summary.steps == 2
    assert summary.checkpoints == 1
    assert summary.rollbacks == 1
    assert len(log.tail(2)) == 2
    assert log.byte_len() > 0


def test_schema_version_is_current(kernel):
    kernel.run("schema goal", 1)
    kernel.save()
    payload = json.loads(ok.Kernel.open(kernel.state_path()).snapshot_json())
    # Schema v6 adds a per-instance case-replay cursor to the v5 safety state.
    assert payload["version"] == 6
    assert payload["case_sample_counter"] >= 0
    assert payload["self_modify_enabled"] is False
    assert "steps_since_last_patch" in payload
    assert "l2_rollback_effect" in payload
    assert "l2_controller" in payload
    assert "l2_pending_policy" in payload
    assert "l2_applied_policy" in payload
    assert "bandit_config" in payload


def test_audit_log_reports_corrupt_records(state_path):
    log = ok.AuditLog.beside_state(str(state_path))
    audit_path = state_path.parent / "audit.jsonl"
    audit_path.write_text("{not-json}\n", encoding="utf-8")

    with pytest.raises(ValueError, match="malformed audit record.*line 1"):
        log.read_all()
    with pytest.raises(ValueError, match="malformed audit record.*line 1"):
        log.poll_after(0)
    with pytest.raises(ValueError, match="malformed audit record.*line 1"):
        log.summarize()


def test_checkpoint_store_skips_corrupt_metadata(state_path):
    checkpoint_dir = state_path.parent / "checkpoints"
    checkpoint_dir.mkdir()
    (checkpoint_dir / "broken.meta.json").write_text("{not-json}", encoding="utf-8")
    store = ok.CheckpointStore.beside_state(str(state_path))
    assert store.list() == []


def test_self_modify_flag_survives_restart(state_path):
    # Enable L1 self-modify via the snapshot, persist, restart, and confirm the
    # switch is still on rather than reverting to the safe default-off.
    kernel = ok.Kernel.with_state_path(str(state_path))
    payload = json.loads(kernel.snapshot_json())
    payload["self_modify_enabled"] = True
    kernel.apply_snapshot_json(json.dumps(payload))
    assert kernel.status().self_modify_enabled is True
    kernel.save()

    reopened = ok.Kernel.open(str(state_path))
    assert reopened.status().self_modify_enabled is True


def test_closed_loop_session_presets_and_external_loop(state_path):
    expected = {
        "deterministic": {"l0": True, "l1": False, "l2": False},
        "hot_patcher": {"l0": False, "l1": True, "l2": False},
        "meta_guard": {"l0": False, "l1": False, "l2": True},
        "fast_adapt": {"l0": True, "l1": True, "l2": False},
        "steady_prior": {"l0": True, "l1": False, "l2": True},
        "control_plane": {"l0": False, "l1": True, "l2": True},
        "full_kernel": {"l0": True, "l1": True, "l2": True},
    }
    for preset, switches in expected.items():
        session = ok.ClosedLoopSession.from_preset(preset, str(state_path))
        assert json.loads(session.switches_json()) == switches

    with pytest.raises(ValueError, match="unknown loop preset"):
        ok.ClosedLoopSession.from_preset("nonexistent_mode", str(state_path))

    det = ok.ClosedLoopSession.from_preset("deterministic", str(state_path))
    d_int = json.loads(det.step_internal_json())
    assert d_int["l1_gate"] == "disabled"
    assert d_int["keep_steady"] is True

    cp = ok.ClosedLoopSession.from_preset("control_plane", str(state_path))
    blocked = json.loads(cp.step_external_json(1, 0.05, 0.10))
    assert blocked["l1_gate"] == "blocked_by_l2"
    assert blocked["rule_patch"] is None
    assert len(blocked["l2_prior"]["kl_prior_bias"]) > 0
    assert "lyapunov_potential" in blocked["l2_prior"]
    assert "selected_expert" in blocked["l2_prior"]
    active = json.loads(cp.step_external_json(2, 0.60, 0.20))
    assert active["l1_gate"] == "active"
    assert active["rule_patch"] is not None

    rolled = json.loads(cp.step_external_json(3, 0.65, 0.20, None, "degraded"))
    assert rolled["rollback_to_checkpoint"] == "ckpt-2"
