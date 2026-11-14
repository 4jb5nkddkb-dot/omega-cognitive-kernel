# Architecture — Omega Cognitive Kernel

**Version:** 0.4.9  
**Positioning:** Training-free online strategy-patch runtime with an auditable control loop.

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Design stance

- **Patch the policy surface**, remember the effect, measure the consequence, roll back if needed.
- No weight updates, no `eval` of arbitrary code.
- The only thing that changes runtime policy is a bounded **`RuleEffect`** (plus soft L2 nudges that still pass through the same sanitize/bounds path).
- L1 patch commit is gated by **`self_modify_enabled`** (default **false**).
- L2 fixed-point / “emergence” language is **heuristic**, not a formal fixed-point theorem.

---

## Layer responsibilities

### L0 — World interaction

| Component | Role |
|-----------|------|
| Observer | Produces observations (simulated or injected adapters) |
| Predictor | Emits uncertainty-aware predictions; applies `error_damping` from active `RuleEffect` |
| DecisionMaker | Scores candidates using **strategy weights** + **RuleEffect** + **L2 meta-bias mapping** + constraints |
| Executor | Whitelist sandbox; rejects unchecked or unknown actions |
| MemoryFacade | Case + Strategy + CAS + active `cognitive_effect` |
| **Learner** | Owns post-step learning: case/strategy/CAS absorb + **idle strategy decay** |

**Invariant:** Actions never leave the executor whitelist. Constraints are checked before execute.

### L1 — Cognitive adaptation

| Component | Role |
|-----------|------|
| Abstractor | Distills trajectory into mean error, variance, success rate, action distribution |
| Reflector | Decides whether a patch is needed (error / variance / success + **cooldown**) |
| PatchGenerator | Builds a `CognitivePatch` with a **structured** `RuleEffect` |
| CodeSandbox | Validates risk, forbidden keywords, and `RuleEffect` bounds |
| VersionedStore | Active module text (audit trail) + history (**capped at 64**) |

**Invariant:** `patch.code` is audit-only. Runtime never executes it. Only `RuleEffect` is applied when `self_modify_enabled`.

### L2 — Meta modulation

| Component | Role |
|-----------|------|
| MetaMonitor | Scalar metrics |
| EmergenceDetector | Heuristic trigger with hysteresis / cooldown |
| OmegaDayanFixedPoint | Soft alignment score |
| MetaModulator | Bias / constraint deltas + soft `RuleEffect` nudge |

**Invariant:** L2 only modulates biases and soft constraints. It cannot issue actions or write CAS.

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

---

## Control loop (one step)

1. **L0:** observe → predict → decide → execute → Learner.update
2. **L1:** distill → reflect → optional patch (if self_modify) → sandbox → commit effect
3. **L2:** metrics → optional fixed-point → biases / soft nudge
4. **Optional:** auto-checkpoint; audit; trajectory.jsonl

---

## Safety summary

| Mechanism | Where |
|-----------|--------|
| **self_modify_enabled** | L1 process gate |
| Action whitelist | L0 executor |
| `max_risk` in scoring | L0 DecisionMaker |
| Risk + keyword + bounds | L1 sandbox |
| Patch cooldown | L1 Reflector |
| Checkpoint before patch | Kernel API |
| Rollback + audit | Kernel API |
| Trajectory dump | Kernel API |

---

## Persistence layout

```
state.json            # KernelSnapshot
audit.jsonl           # append-only events
trajectory.jsonl      # optional step dump
checkpoints/          # FIFO snapshots
```

---

## Public vs internal API

| Supported for integrators | Treat as unstable |
|---------------------------|-------------------|
| `Kernel`, `KernelStatus`, `RunReport` | `kernel.inner.*` field layout |
| Config / adapters / checkpoint / audit | Heuristic thresholds inside L2 |
| CLI + optional `omega-dashboard` | Example-only demos |

See `docs/API_FREEZE_0.4.md` for the 0.4.x freeze list.
