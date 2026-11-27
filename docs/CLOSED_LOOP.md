# Omega Closed-Loop Checklist (2026-11-26)

## Product status

| Item | Status | Notes |
|------|--------|-------|
| Main cognitive cycle | ✅ | observe → intent → predict → decide → execute → CAS/strategy → meta |
| Persistent state | ✅ | `state.json` |
| Strategy + case memory | ✅ | |
| Audit trail | ✅ | `audit.jsonl` |
| Checkpoints / rollback | ✅ | `Kernel::rollback`, `cli_demo` end-to-end test, and `omega-kernel rollback` state persistence |
| **Self-modify master switch** | ✅ | `self_modify_enabled = false` by default |
| L1 Shadow mode | ✅ | `l1_shadow_mode = false` by default; dry-run validation without mutating `RuleEffect` |
| L1/L2 write-mutex arbitration | ✅ | `L2MetaLayer::blocks_l1_write()` enforces L2 priority during active gate, defensive mode, committed policy, or cooldown |
| L1 auto-rollback | ✅ | `OmegaOrchestrator::run_once` observes `error_delta > 0.05` and restores `previous_effect` |
| BodyBus | ✅ | `src/body/mod.rs`: `no_std` sensor/actuator channels, `write_authorized` always `false` |
| Trajectory dump | ✅ | `trajectory.jsonl` when `dump_trajectory=true` |
| CLI status shows switch | ✅ | `omega-kernel status` |
| Weekly runtime report | ✅ | `.github/workflows/weekly-runtime-report.yml` |
| Standalone audit product | ✅ | `omega-audit-panel` binary |

## Config (omega.toml)

```toml
self_modify_enabled = false
l1_shadow_mode = false
dump_trajectory = true
autosave = true
auto_checkpoint_on_patch = true
error_threshold = 0.30
min_steps_between_patches = 3
max_risk = 0.65

[intervention]
mode = "exploration"
max_level = "suggest_alternative"
friction_threshold = 2
audit_escalations = true
```

## Commands

```bash
# main runtime
cargo run --release --bin omega-kernel -- status
cargo run --release --bin omega-kernel -- run --steps 12

# standalone vertical product
cargo run --release --bin omega-audit-panel
cargo run --release --bin omega-audit-panel -- ./path/to/state.json
```

## Safe procedure

1. Keep `self_modify_enabled = false` until you deliberately want L1 rule evolution; enable `l1_shadow_mode = true` first (keeping `self_modify_enabled = false`) for dry-run observation before carefully enabling L1 self-modification.
2. Inspect `status`, `audit --summary`, and `trajectory.jsonl` after runs.
3. Weekly Actions writes `reports/weekly-YYYY-Www.md` automatically.
