# API notes — 0.6.x

**Source baseline:** `0.6.0`
**On-disk state schema:** `2`
**API status:** the 0.6 surface is implemented in the current source tree. L1/L2 heuristics and the exact trajectory payload remain experimental.

This document describes the public Rust API, persisted state, audit/event stream, and compatibility expectations. It complements [`INTEGRATING.md`](INTEGRATING.md) and [`ARCHITECTURE.md`](../ARCHITECTURE.md).

## Public kernel surface

The product-facing type is generic over the observer, decision maker, and executor:

```text
Kernel<O, D, E>
  O: Observer
  D: DecisionMaker
  E: Executor
```

The predictor is `ModelBasedPredictor`. The default concrete type uses `MultiModalObserver`, `ConstrainedDecisionMaker`, and `SandboxedExecutor`. `BanditKernel` uses `BanditUcbDecisionMaker` as `D`.

Primary constructors and lifecycle methods:

```text
Kernel::new() -> Kernel<MultiModalObserver, ConstrainedDecisionMaker, SandboxedExecutor>
Kernel::with_state_path(path) -> Kernel<...>
Kernel::open(path: Option<&str>) -> Result<Kernel<...>>
Kernel::with_adapters(observer, executor, decision) -> Kernel<O, D, E>
Kernel::with_adapters_and_path(observer, executor, decision, path) -> Kernel<O, D, E>
Kernel::with_bandit_ucb(path) -> BanditKernel
Kernel::from_config_path(path) -> Result<Kernel<...>>       # feature = "toml"
Kernel::step(goal) -> Result<StepResult>
Kernel::run(goal, steps) -> Result<RunReport>
Kernel::status() -> KernelStatus
Kernel::save() / load() / reset()
```

Operational and recovery methods include `checkpoint`, `list_checkpoints`, `rollback`, `audit_tail`, `audit_summary`, `replay_trajectory`, `snapshot`, `snapshot_json`, `apply_snapshot`, and `apply_snapshot_json`.

## Safety controls

The runtime has two intentionally independent mutation channels:

| Channel | Public control | Default | Effect |
|---|---|---:|---|
| L1 discrete patch | `self_modify_enabled` / `KernelConfig.self_modify_enabled` | `false` | Allows a validated `CognitivePatch` to install a new `RuleEffect` and active-module version |
| L2 continuous modulation | `set_l2_enabled(bool)` | `false` | Allows bounded bias/constraint/effect modulation while metric monitoring remains available |

`self_modify_enabled = false` does not disable L2. To freeze all policy mutation, keep both gates disabled (the default). Disabling L2 clears its fixed-point state and dwell counter but still evaluates metrics for comparable baselines.

L1 patch candidates are distilled, reflected, sanitized, validated by `CodeSandbox`, and subject to a minimum step cooldown. The behavior-changing surface is structured `RuleEffect`; arbitrary patch code is not executed.

## Schema v2 and migration

`CURRENT_SCHEMA_VERSION` is `2`. `KernelSnapshot` persists the following groups of state:

- `total_steps`, `biases`, `constraints`, CAS totals/error/adaptation score.
- The active `RuleEffect`.
- Case memory, case capacity, strategy memory, active modules, and patch history.
- L2 enablement, fixed-point/dwell/alignment state, commit threshold/cooldown, consequence and defensive state, detector history/configuration, and negative-reward tracking.
- L1 `self_modify_enabled` and `steps_since_last_patch`.
- Optional `l2_rollback_effect`, the pre-commit effect used for negative-feedback rollback.

Loading accepts older snapshots and calls `KernelSnapshot::migrate`:

```text
v0 or missing version -> v1 -> v2
v1 -> v2 with safe defaults
v2 -> current state
newer than v2 -> error
```

For the v1-to-v2 transition, missing safety fields default to the safe behavior: L1 disabled, a warm patch cooldown, and no saved L2 rollback effect. A future schema bump must add an explicit migration path and regression tests before release.

State saves create the parent directory, serialize version `2`, write a same-directory temporary file, call `sync_all`, and atomically rename it over the target. Failed replacement removes the temporary file and preserves the previous target where the operating system permits it.

## L2 API and consequence feedback

`L2MetaLayer::step` returns an `L2Result` containing:

| Field | Meaning |
|---|---|
| `metrics` | Current `MetaMetrics` evaluation |
| `in_fixed_point` | Whether the layer remains in the fixed-point regime |
| `entered_fixed_point` / `exited_fixed_point` | Regime transition flags |
| `alignment_score` / `candidate_alignment_score` | Current and candidate alignment values |
| `dwell_steps` | Consecutive steps in the fixed-point regime |
| `candidate_score` / `score_breakdown` | Candidate modulation score and components |
| `modulation_proposed` | A candidate was evaluated |
| `modulation_committed` | The candidate passed commit checks and was applied |
| `modulation_gain` | Bounded scale applied to the modulation |
| `effect_nudge` | Optional bounded `RuleEffect` update |
| `biases` / `constraints` | Updated L0 maps returned to the orchestrator |

By default, L2 uses a commit threshold of `0.82` and a five-step commit cooldown. A commit is additionally blocked by defensive mode and consequence-stability conditions. On commit, the previous effect is stored in `rollback_effect` and the layer waits for feedback.

The product API feeds back the short-term consequence with:

```rust
kernel.observe_l2_consequence(reward_delta, error_delta, risk_delta);
```

The internal consequence score is:

```text
0.60 * reward_delta - 0.25 * error_delta - 0.15 * risk_delta
```

A score below `-0.05` marks the outcome negative, extends cooldown, exits fixed-point mode, and can enter defensive mode after repeated negative consequences. `observe_l2_consequence` restores the saved effect through `take_rollback_effect` when a negative outcome qualifies. Positive outcomes accumulate stable consequence steps and can recover defensive mode.

## Status export

`KernelStatus` is serializable and is exposed by `Kernel::status()` and the CLI `status --json`. Its current fields are:

```text
schema_version
 total_steps
avg_prediction_error
adaptation_score
error_damping
explore_bonus
risk_tolerance
active_modules
strategy_count
case_count
state_path
checkpoint_count
in_fixed_point
dwell_steps
last_alignment_score
max_risk
self_modify_enabled
l2_enabled
l2_commit_cooldown
last_l2_consequence_score
l2_defensive_mode
l2_negative_consequence_streak
```

Consumers should use named JSON fields and tolerate additive fields in future versions.

## Audit and event stream

`KernelEvent` is an alias of `AuditEvent`. The append-only `audit.jsonl` stream currently includes:

```text
Step
PatchAccepted
PatchRejected
Checkpoint
Rollback
TaskEpisode
ToolAllowed
ToolDenied
FixedPointEntered
FixedPointExited
L2ModulationProposed
L2ModulationCommitted
```

The in-process API is:

```text
Kernel::set_event_hook(FnMut(&AuditEvent) + Send + 'static)
Kernel::clear_event_hook()
```

A hook fires only after the related audit record has been appended successfully. `AuditLog::byte_len()` returns a cursor position and `AuditLog::poll_after(offset)` returns complete records plus the next cursor. An incomplete trailing JSONL line is held for a later poll; if the file was truncated or rotated, polling resynchronizes from offset zero.

The `Kernel::step` event ordering is intentionally observable:

```text
Step
  -> fixed-point transition events, if any
  -> L2 proposal/commit events, if any
  -> trajectory append, if enabled
  -> checkpoint event, if a pre-patch checkpoint succeeded
  -> PatchAccepted or PatchRejected, if applicable
  -> autosave, if enabled
```

Audit append, trajectory write, and autosave errors are returned to the caller. A failed optional pre-patch checkpoint does not suppress the primary `PatchAccepted` event; if its own audit append fails, that failure is returned.

## Offline trajectory replay

`replay_trajectory_file(path, tail_n)` and `Kernel::replay_trajectory(tail_n)` read `trajectory.jsonl` without re-running the external world. The summary exposes:

```text
path
rows
patches
first_avg_err
last_avg_err
action_counts
tail: step, action, patched, avg_err, goal
```

The exact trajectory JSON field set may grow. Consumers should ignore unknown fields and treat missing optional fields as unavailable.

## Strategy weights and CLI additions

```text
Kernel::set_strategy_weight(name, weight: f64) -> bool
```

The method returns `false` for an unknown strategy. The CLI equivalents include:

```bash
omega-kernel strategy-weight explore 1.25
omega-kernel audit-poll --offset 0
omega-kernel status --json
omega-kernel replay --tail 15
```

## Compatibility contract

Stable compatibility surfaces are the public constructors/methods documented here, `KernelStatus` named fields, schema migration, and additive tagged audit events. The following remain experimental or intentionally unstable:

- `kernel.inner.*` layout.
- L1/L2 internal heuristics and exact scores.
- Exact trajectory JSON field set.
- Exact `AuditEvent` fields beyond the documented tagged variants.
- Dashboard internal state and workspace implementation details.

Consumers should not depend on internal module layout or infer policy success solely from `PatchAccepted`/`L2ModulationCommitted`; evaluate consequence, risk, error, rollback, and benchmark results separately.
