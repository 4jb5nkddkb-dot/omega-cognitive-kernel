# Omega Closed-Loop Checklist (2026-11-12)

## What is now product-ready

| Item | Status | Notes |
|------|--------|-------|
| Main cognitive cycle (observe → intent → predict → decide → execute → CAS/strategy → meta) | ✅ | `Kernel::step` / `run` |
| Persistent state | ✅ | `state.json` beside process |
| Strategy + case memory | ✅ | `memory/strategy.rs`, `memory/case.rs` |
| Audit trail | ✅ | `audit.jsonl` |
| Checkpoints / rollback | ✅ | UUID checkpoints |
| **Self-modify master switch** | ✅ | `self_modify_enabled = false` by default in `omega.toml` |
| Trajectory dump | 🔧 | Config flag `dump_trajectory` added; wire-up in progress |
| Intervention policy | ✅ | exploration vs strict |

## Config knobs (omega.toml)

```toml
self_modify_enabled = false   # MUST stay false until you explicitly want L1 patches
dump_trajectory = true        # append full step trajectory for replay
autosave = true
auto_checkpoint_on_patch = true
error_threshold = 0.30
min_steps_between_patches = 3
max_risk = 0.65

[intervention]
mode = "exploration"          # or "strict"
max_level = "suggest_alternative"
friction_threshold = 2
audit_escalations = true
```

## Safe operating procedure

1. Run with defaults (`self_modify_enabled = false`).
2. Inspect `audit` + status after a few `run` sessions.
3. Only flip `self_modify_enabled = true` when you want controlled rule evolution.
4. Every accepted patch still goes through sandbox validation + optional pre-patch checkpoint.

## Next engineering tasks

1. Finish trajectory JSONL writer in `Kernel::step` (config already present).
2. Expose `self_modify_enabled` in CLI status line.
3. Weekly Actions report of patch counts / fixed-point dwell / strategy stats.
4. Productize one vertical (inbox / tool-gate / audit panel) as a standalone binary.

## Relation to file-seal

`file-seal` is a complete offline integrity product (GUI + CLI).  
Omega is the cognitive runtime; use the same pattern: local-first, explicit switches, full audit.
