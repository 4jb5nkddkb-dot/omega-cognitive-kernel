# API notes — 0.5.x

**Status:** in effect as of **v0.5.0**.

0.4 freeze is **lifted** for the items below. Unlisted 0.4 surfaces remain supported unless called out in CHANGELOG.

## Breaking / additive vs 0.4

| Item | Change |
|------|--------|
| `KernelStatus` | **New field** `schema_version: u32` (serde consumers: additive) |
| `KernelSnapshot.version` | Missing / 0 → migrated to 1 on load; unknown higher versions **error** |
| CLI | New subcommand `replay` |
| Public exports | `replay_trajectory_file`, `TrajectoryReplaySummary`, `TrajectoryReplayRow`, `CURRENT_SCHEMA_VERSION` |

## Stable integrator surface (0.5)

Same as 0.4 freeze list, plus:

- `CURRENT_SCHEMA_VERSION`
- `KernelSnapshot::migrate`
- `replay_trajectory_file(path, tail_n)`
- `Kernel::replay_trajectory(tail_n)`

## State migration

```text
load state.json
  → deserialize (version default 0 if absent)
  → migrate()
      version > CURRENT → error
      version < 1       → case_capacity default 256, version = 1
  → apply_snapshot (CaseMemory::restore backfills case.priority)
```

## Replay contract

Replay **does not** re-execute the world. It only summarizes `trajectory.jsonl`:

- row count, patch count
- first/last `avg_err`
- action histogram
- last N step rows

## Still unstable

- `kernel.inner.*` layout
- L1/L2 internal heuristics
- Exact trajectory JSON field set (may grow)
