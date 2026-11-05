# Architecture — Omega Cognitive Kernel

**Version:** 0.3.3  
**Positioning:** Training-free online strategy-patch runtime with auditable control loop.

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Design stance

- **Patch the policy surface**, remember the effect, measure the consequence, roll back if needed.
- No weight updates, no `eval` of arbitrary code.
- The only thing that changes runtime behavior is a bounded `RuleEffect`.

---

## Layer responsibilities

### L0 — World interaction

| Component | Role |
|-----------|------|
| Observer | Produces observations (currently simulated structured sensor) |
| Predictor | Emits uncertainty-aware predictions; applies `error_damping` from active `RuleEffect` |
| DecisionMaker | Scores candidate actions using strategy weights + `action_preference` + `explore_bonus` + uncertainty vs `risk_tolerance` |
| Executor | Whitelist sandbox; rejects unchecked or unknown actions |
| MemoryFacade | Case + Strategy + CAS + active `cognitive_effect` |

**Invariant:** Actions never leave the executor whitelist. Constraints are checked before execute.

### L1 — Cognitive adaptation

| Component | Role |
|-----------|------|
| Abstractor | Distills trajectory into mean error, variance, success rate, action distribution |
| Reflector | Decides whether a patch is needed (error / variance / success + **cooldown**) |
| PatchGenerator | Builds a `CognitivePatch` with a **structured** `RuleEffect` (strategy-aware) |
| CodeSandbox | Validates risk, forbidden keywords, and `RuleEffect` bounds |
| VersionedStore | Keeps active module code (audit trail) + history (**capped at 64**) |

**Invariant:** `patch.code` is audit-only. Runtime never executes it. Only `RuleEffect` is applied to memory.

**Cooldown:** After an accepted patch, Reflector ignores new candidates for `min_steps_between_patches` (default 3) steps to avoid thrashing.

### L2 — Meta modulation

| Component | Role |
|-----------|------|
| MetaMonitor | Produces scalar metrics; **consistency is data-driven** from prediction error + code presence |
| EmergenceDetector | Heuristic trigger with **hysteresis / cooldown**; also provides soft exit signal |
| OmegaDayanFixedPoint | Soft alignment score (omega ≈ dayan / self-reference) enriched with error maturity |
| MetaModulator | Applies small bias / constraint deltas |

**Invariant:** L2 only modulates biases and soft constraints. It cannot issue actions or write CAS.

**Fixed-point lifecycle:** Entry requires sustained high predictive / self-ref metrics. Exit occurs on metric degradation or long dwell without re-alignment.

---

## RuleEffect (the only mutable policy surface)

```rust
struct RuleEffect {
    error_damping: f64,      // multiplies uncertainty  [0.2, 1.5]
    action_preference: Map,  // additive scores         [-5, 5] per allowed action
    min_uncertainty: f64,    // floor after damping     [0.01, 0.5]
    explore_bonus: f64,      // score bias for explore  [-1.5, 1.5]
    risk_tolerance: f64,     // high-uncertainty threshold [0.15, 0.85]
}
```

- Produced by L1, sanitized, bounds-checked by sandbox.
- Consumed by L0 predictor and decision.
- Persisted inside `KernelSnapshot`.

---

## Control loop (one step)

1. L0: observe → predict (with current effect) → decide → execute → update memory
2. L1: distill (rich features) → reflect (with strategy / error context + cooldown) → optional patch → sandbox → commit effect
3. L2: evaluate metrics → optional fixed-point entry / dwell / exit → update biases / constraints
4. Optional: auto-checkpoint before accepted patch; audit every step

---

## Safety summary

| Mechanism | Where |
|-----------|--------|
| Action whitelist | L0 executor |
| Constraints-checked flag | L0 decision → executor |
| Risk + keyword + bounds | L1 sandbox |
| Structured effect only | L1 → memory |
| Patch cooldown | L1 Reflector |
| History bound | L1 VersionedStore (64) |
| Checkpoint before patch | Kernel API |
| Rollback + audit | Kernel API |

There is no path from L1/L2 that can execute arbitrary code or bypass the executor whitelist.

---

## Persistence layout

```
.omega/
  state.json          # KernelSnapshot
  audit.jsonl         # append-only events
  checkpoints/        # FIFO snapshots (max 32)
```

---

## What this is not

- Not a full AGI architecture
- Not a weight-training loop
- Not a general code-execution sandbox for untrusted source
- Not a replacement for the heavier Python `omega-core-grok` safety pipeline

It is a minimal, auditable, training-free adaptation kernel that can demonstrate measurable self-modification consequence under external reward.
