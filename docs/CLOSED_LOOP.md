# Omega Closed-Loop Checklist (2026-11-12)

## Product status

| Item | Status | Notes |
|------|--------|-------|
| Main cognitive cycle | ✅ | observe → intent → predict → decide → execute → CAS/strategy → meta |
| Persistent state | ✅ | `state.json` |
| Strategy + case memory | ✅ | |
| Audit trail | ✅ | `audit.jsonl` |
| Checkpoints / rollback | ✅ | |
| **Self-modify master switch** | ✅ | `self_modify_enabled = false` by default |
| Trajectory dump | ✅ | `trajectory.jsonl` when `dump_trajectory=true` |
| CLI status shows switch | ✅ | `omega-kernel status` |
| Weekly runtime report | ✅ | `.github/workflows/weekly-runtime-report.yml` |
| Standalone audit product | ✅ | `omega-audit-panel` binary |

## Config (omega.toml)

```toml
self_modify_enabled = false
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

1. Keep `self_modify_enabled = false` until you deliberately want L1 rule evolution.
2. Inspect `status`, `audit --summary`, and `trajectory.jsonl` after runs.
3. Weekly Actions writes `reports/weekly-YYYY-Www.md` automatically.

## Relation to file-seal

`file-seal` = offline integrity desktop product (closed).
Omega = cognitive runtime with explicit safety switches and operator panel.
