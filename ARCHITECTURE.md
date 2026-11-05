# Architecture — Omega Cognitive Kernel

**Version:** 0.3.7  
**Positioning:** Training-free online strategy-patch runtime with an auditable control loop.

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Design stance

- **Patch the policy surface**, remember the effect, measure the consequence, roll back if needed.
- No weight updates, no `eval` of arbitrary code.
- The only thing that changes runtime policy is a bounded **`RuleEffect`** (plus soft L2 nudges that still pass through the same sanitize/bounds path).
- L2 fixed-point / “emergence” language is **heuristic**, not a formal fixed-point theorem.

---

## Layer responsibilities

### L0 — World interaction

| Component | Role |
|-----------|------|
| Observer | Produces observations (currently simulated structured sensor) |
| Predictor | Emits uncertainty-aware predictions; applies `error_damping` from active `RuleEffect` |
| DecisionMaker | Scores candidates using **strategy weights** + **RuleEffect** + **L2 meta-bias mapping** + constraints |
| Executor | Whitelist sandbox; rejects unchecked or unknown actions |
| MemoryFacade | Case + Strategy + CAS + active `cognitive_effect` |
| **Learner** | Owns post-step learning: case/strategy/CAS absorb + **idle strategy decay** |

**Invariant:** Actions never leave the executor whitelist. Constraints are checked before execute.

#### Decision scoring (summary)

For each candidate action (`observe`, `explore`, `exploit`, `stabilize`):

1. Strategy weight (from strategy memory)
2. `RuleEffect.action_preference`
3. Direct action-named biases (if any)
4. **L2 meta-bias deltas** (see below)
5. `explore_bonus` / `risk_tolerance` / `max_risk` pressure
6. Uncertainty regime (high unc → prefer stabilize/observe)

#### L2 meta-bias → action mapping

| Bias key | Effect on scores |
|----------|------------------|
| `explore_preference` | Boost explore; mild penalty on exploit/stabilize |
| `self_ref` + `coherence` | Boost stabilize (+ mild observe); mild penalty on explore |
| `observation_depth` | Boost observe |

These biases are produced by L2 and carried into L0 via `Intent.bias_from_memory`.

---

### L1 — Cognitive adaptation

| Component | Role |
|-----------|------|
| Abstractor | Distills trajectory into mean error, variance, success rate, action distribution |
| Reflector | Decides whether a patch is needed (error / variance / success + **cooldown**) |
| PatchGenerator | Builds a `CognitivePatch` with a **structured** `RuleEffect` (strategy-aware) |
| CodeSandbox | Validates risk, forbidden keywords, and `RuleEffect` bounds |
| VersionedStore | Active module text (audit trail) + history (**capped at 64**) |
| Learner (hook) | Available for post-patch learning; primary update path lives on L0 |

**Invariant:** `patch.code` is audit-only. Runtime never executes it. Only `RuleEffect` is applied to memory.

**Cooldown:** After an accepted patch, Reflector ignores new candidates for `min_steps_between_patches` (default 3) steps to avoid thrashing.

---

### L2 — Meta modulation

| Component | Role |
|-----------|------|
| MetaMonitor | Scalar metrics; **consistency is data-driven** (prediction error + code presence) |
| EmergenceDetector | Heuristic trigger with **hysteresis / cooldown**; soft exit signal |
| OmegaDayanFixedPoint | Soft alignment score (self-reference / naming overlap + error maturity) |
| MetaModulator | Bias / constraint deltas + **soft `RuleEffect` nudge** (`explore_bonus` / `risk_tolerance` deltas, then sanitize) |

**Invariant:** L2 only modulates biases and soft constraints (and soft-nudges `RuleEffect` through the same bounds path). It cannot issue actions or write CAS.

**Fixed-point lifecycle (heuristic):**

1. **Entry** — sustained high predictive / self-ref style metrics  
2. **Dwell** — track `dwell_steps` / alignment  
3. **Exit** — metric degradation or long dwell without re-alignment  

Orchestrator merges any L2 `effect_nudge` into `memory.cognitive_effect` for the next L0 step.

---

## RuleEffect (the only mutable policy surface)

```text
RuleEffect {
    error_damping:     f64,   // multiplies uncertainty     [0.2, 1.5]
    action_preference: Map,   // additive scores            [-5, 5] per allowed action
    min_uncertainty:   f64,   // floor after damping        [0.01, 0.5]
    explore_bonus:     f64,   // score bias for explore     [-1.5, 1.5]
    risk_tolerance:    f64,   // high-uncertainty threshold [0.15, 0.85]
}
```

- Produced by L1 (and soft-nudged by L2), always **sanitized** and bounds-checked.
- Consumed by L0 predictor and decision.
- Persisted inside `KernelSnapshot`.

---

## Strategy memory & idle decay

Each strategy tracks `weight`, success/fail counts, and **`idle_steps`**.

- On absorb: all strategies age; the chosen action resets idle and bumps weight.
- **Learner** (L0) calls `decay_idle(threshold, factor)` so long-idle arms slowly lose weight.
- Prevents early lucky weights from permanently locking the policy.

**Observability:** `StrategyMemory::stats` exposes counts and mean weights for active vs idle arms. The bandit task report includes:

- `strategy_count`, `idle_strategy_count`
- `mean_strategy_weight`, `mean_idle_weight`, `mean_active_weight`
- `mean_idle_steps`, `max_idle_steps`

---

## Control loop (one step)

1. **L0:** observe → predict (current effect) → decide (effect + L2 biases + constraints) → execute → **Learner.update** (case/strategy/CAS + idle decay)
2. **L1:** distill → reflect (strategy/error context + cooldown) → optional patch → sandbox → commit effect
3. **L2:** metrics → optional fixed-point entry / dwell / exit → update biases / constraints → optional soft RuleEffect nudge
4. **Optional:** auto-checkpoint before accepted patch; audit every step

---

## Safety summary

| Mechanism | Where |
|-----------|--------|
| Action whitelist | L0 executor |
| Constraints-checked flag | L0 decision → executor |
| `max_risk` in scoring | L0 DecisionMaker |
| Risk + keyword + bounds | L1 sandbox |
| Structured effect only | L1 → memory; L2 nudge re-sanitized |
| Patch cooldown | L1 Reflector |
| History bound | L1 VersionedStore (64) |
| Checkpoint before patch | Kernel API |
| Rollback + audit | Kernel API |
| L2 cannot act / write CAS | L2MetaLayer invariant |

There is no path from L1/L2 that can execute arbitrary code or bypass the executor whitelist.

---

## Persistence layout

```
.omega/
  state.json          # KernelSnapshot
  audit.jsonl         # append-only events
  checkpoints/        # FIFO snapshots
```

---

## Public vs internal API

| Supported for integrators | Treat as unstable |
|---------------------------|-------------------|
| `Kernel`, `KernelStatus`, `RunReport` | `kernel.inner.*` field layout |
| `run_bandit_task` / `run_bandit_task_seeded` | Concrete L0/L1/L2 struct fields |
| Checkpoint / rollback / audit helpers | Heuristic thresholds inside L2 |

---

## What this is not

- Not a full AGI architecture
- Not a weight-training loop
- Not a general code-execution sandbox for untrusted source
- Not a formal proof of fixed-point / emergence
- Not a replacement for a heavier multi-process safety pipeline (e.g. a separate Python core)

It is a minimal, auditable, training-free adaptation kernel that can demonstrate **measurable self-modification consequence** under external reward, with reversible state and explicit safety boundaries.
