# Architecture — Omega Cognitive Kernel

**Source baseline:** `0.6.0`
**State schema:** `6`
**Positioning:** Training-free online strategy-patch runtime with an auditable control loop.

```text
World (L0) -> Reflect & patch (L1) -> Meta-modulate (L2) -> bias next L0
```

## Design stance

Omega patches a bounded policy surface, remembers the effect, measures the consequence, and rolls back when needed. It does not update model weights or evaluate arbitrary code as a policy mechanism. The behavior-changing policy object is the structured `RuleEffect`.

The runtime is designed around four invariants:

1. **Bounded mutation:** `RuleEffect::sanitize` clamps numeric fields and removes unknown action preferences.
2. **Explicit gates:** L1 and L2 mutation channels have separate switches and safe defaults.
3. **Durable observability:** audit, trajectory, checkpoint, and snapshot failures are not silently ignored.
4. **Offline inspectability:** trajectory replay and audit polling do not re-execute the external world.

## Generic runtime composition

The public `Kernel<O, D, E>` fixes the predictor to `ModelBasedPredictor` while injecting observer, decision maker, and executor boundaries:

```text
Kernel<O, D, E>
  -> OmegaOrchestrator<O, ModelBasedPredictor, D, E>
     -> L0WorldLayer<O, ModelBasedPredictor, D, E>
     -> L1CognitiveLayer
     -> L2MetaLayer
```

The defaults are `MultiModalObserver`, `ConstrainedDecisionMaker`, and `SandboxedExecutor`. `BanditKernel` substitutes `BanditUcbDecisionMaker` for standard two-armed bandit decisions; `Kernel::with_bandit_ucb(path)` constructs that specialization.

`OmegaOrchestrator` also owns the current L0 bias map, constraint map, and total step counter. `MemoryFacade` belongs to L0 and composes case memory, strategy memory, CAS metrics, and the active `RuleEffect`.

## One-step control loop

`Kernel::step(goal)` takes an optional pre-step snapshot, then calls the orchestrator. The orchestrator executes:

```text
1. Build Intent(goal, priority=1.0, bias_from_memory)
2. L0WorldLayer::step
   a. Observer::observe
   b. MemoryFacade::retrieve(goal, k=5)
   c. Predictor::predict
   d. DecisionMaker::decide under constraints
   e. Executor::execute
   f. Compute prediction error from uncertainty + action term
   g. Learner::update cases, strategies, and CAS
3. L1CognitiveLayer::process trajectory
4. Build SystemState from active modules and CAS metrics
5. L2MetaLayer::step
6. Install returned biases, constraints, and optional effect nudge
7. Persist audit events, trajectory, checkpoints, and state
```

L0 prediction error is clamped to `[0.05, 1.0]`. `stabilize` and `exploit` contribute a small negative action-quality term, while `explore` contributes a positive term. The exact simulation behavior is not a guarantee for real-world adapters.

## Two independent mutation gates

`RuleEffect` can be changed through two separate channels:

| Channel | What changes | Gate | Default |
|---|---|---|---:|
| **L1 patch** | Discrete `CognitivePatch` and versioned active-module/effect commit | `self_modify_enabled` | `false` |
| **L2 modulation** | Validated expert/KL-prior policy snapshot for a subsequent L0 decision | `l2.enabled` | `true` |

L1 increments its patch cooldown, distills the trajectory, and stops with `self_modify_enabled=false` when disabled. When enabled, it builds reflection context from CAS error, strategy weights, and recent case errors; sanitizes and sandbox-validates the candidate; then installs the effect and commits the versioned store. Rejected candidates do not change the active effect.

L2 continues metric evaluation when disabled, but returns no intervention and clears fixed-point dwell state. New kernels enable L2 by default while L1 remains disabled. Call `set_l2_enabled(false)` for a monitoring-only baseline.

## L2 fixed-point and consequence loop

`L2MetaLayer` contains a `MetaMonitor`, `EmergenceDetector`, `OmegaDayanFixedPoint`, and a resident controller. The controller maintains trust, discounted-Hedge/IPS experts, an online Dayan residual, a bounded Lyapunov certificate, and a small context cache. It tracks:

```text
metrics
in_fixed_point / dwell_steps
last_alignment_score
commit_threshold / cooldown
last_consequence_score
negative_consequence_streak
stable_consequence_steps
defensive_mode
awaiting_consequence
reward/error change scores
rollback_effect
```

A proposal is executable only when the active expert, trust, model confidence, cooldown, defensive-mode, and Lyapunov gates all permit it. The immutable committed `L2Policy` supplies a prior and beta for the next cycle's KL-regularized action distribution; it does not directly alter reward estimates or bypass L0 dwell/margin/safety checks. Invalid, dormant, held, vetoed, malformed, and `KEEP_L0` snapshots collapse to zero intervention. For a frozen Dayan target, residual relaxation is `r+ = (1 - lambda)r`; this contraction claim does not extend to a moving target or to external task recovery.

`observe_l2_consequence(reward_delta, error_delta, risk_delta)` computes:

```text
consequence_score =
    0.60 * reward_delta
  - 0.25 * error_delta
  - 0.15 * risk_delta
```

A score below `-0.05` marks a negative consequence, extends cooldown, leaves fixed-point mode, and can activate defensive mode after repeated negatives. The saved `rollback_effect` is returned when the negative consequence qualifies. Positive consequences increase stable consequence steps, clear negative streaks, and can recover defensive mode after a strong enough result.

## Persistence model and schema v6

The default state path is `.omega/state.json`. Sidecars are placed beside it:

```text
.omega/
├── state.json          # KernelSnapshot, CURRENT_SCHEMA_VERSION = 6
├── audit.jsonl         # append-only AuditRecord stream
├── trajectory.jsonl    # optional per-step replay source
└── checkpoints/        # complete rollback snapshots
```

`KernelSnapshot` persists total steps, biases, constraints, CAS metrics, `RuleEffect`, cases, strategies, active modules, patch history, L1 self-modification/cooldown state, UCB decision state, the resident L2 controller, bounded cache, Dayan residual, pending/applied policy snapshots, and a per-instance case-replay cursor.

Loading migrates v0 or missing-version snapshots through v1, v2, v3, v4, and v5 to v6. Missing legacy safety fields default to L1/L2 disabled, a warm patch cooldown, no rollback effect, and `KEEP_L0` policy snapshots. Migrating with L2 disabled clears any restored pending/applied intervention and forces the controller dormant. A snapshot with a version newer than the supported schema is rejected. A schema bump requires a migration function, compatibility tests, and release documentation.

Saving serializes schema v6 to a same-directory temporary file, calls `sync_all`, and atomically renames it over the state path. Temporary files are removed on failure. Audit append, trajectory write, and autosave errors propagate through `Kernel::step`; a step must not report success after dropping a durable transition. External execution happens before these writes, so a returned error cannot imply that an executor's side effect was rolled back.

## Audit and replay ordering

The audit event stream is the public `KernelEvent` alias and includes step, patch, checkpoint, rollback, task, tool-gate, fixed-point, and L2 modulation events. During a step, the durable sequence is:

```text
Step
-> FixedPointEntered / FixedPointExited, if applicable
-> L2ModulationProposed / L2ModulationCommitted, if applicable
-> trajectory append, if enabled
-> pre-patch Checkpoint, if saved successfully
-> PatchAccepted or PatchRejected, if applicable
-> autosave, if enabled
```

In-process event hooks fire only after each individual audit append succeeds. External consumers can use `AuditLog::byte_len` and `poll_after` to read complete JSONL records from a byte cursor; incomplete trailing lines are retained for a later poll and file truncation causes resynchronization. Malformed complete records return a path, line, and byte-offset error rather than being silently discarded.

Trajectory replay summarizes existing JSONL rows without re-executing the world. It reports row/patch counts, first/last average error, action counts, and a tail of step/action/patch/error/goal fields.

## Feature and portability boundaries

The crate is Rust 2021 with MSRV `1.74`. The `std` feature gates product-facing filesystem, audit, configuration, task, and persistence modules. Optional feature groups provide TOML, `ureq`, `reqwest`, GUI, async adapters, WASM support, and docs.rs attributes. The `msrv` feature intentionally avoids dependency combinations that require Rust 2024-era transitive packages.

The portability goal is to keep core data types and orchestration usable in reduced environments, while filesystem persistence, network adapters, GUI, and some task utilities remain host-feature concerns.

## Compatibility boundaries

Stable design contracts are the bounded `RuleEffect` surface, independent L1/L2 gates, schema migration, fail-closed persistence, named `KernelStatus` fields, and additive tagged audit events. The following remain experimental and must not be treated as stable integration contracts:

- `kernel.inner.*` layout.
- Exact L1/L2 heuristic formulas and thresholds beyond documented defaults.
- Exact trajectory JSON field set.
- Internal dashboard/workspace structures.
- Benchmark outcomes before reproducible evidence establishes a benefit.

See [`docs/API_0.5.md`](docs/API_0.5.md) for the public API and [`docs/INTEGRATING.md`](docs/INTEGRATING.md) for embedding guidance.
