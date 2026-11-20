# Architecture — Omega Cognitive Kernel

**Version:** 0.5.1  
**Positioning:** Training-free online strategy-patch runtime with an auditable control loop.

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Design stance

- **Patch the policy surface**, remember the effect, measure the consequence, roll back if needed.
- No weight updates, no `eval` of arbitrary code.
- The only mutable policy surface is a bounded **`RuleEffect`**.
- On-disk state is **schema-versioned** (`CURRENT_SCHEMA_VERSION`).
- Trajectory can be **replayed offline** (CLI + dashboard) without world re-execution.

## Two independent self-modification gates

`RuleEffect` can be changed through **two separate channels**, each with its own
switch. They are intentionally independent:

| Channel | What changes | Gate | Default |
|---------|--------------|------|---------|
| **L1 patch** | Discrete structured `CognitivePatch` commit to the `cognitive_rules` module (the riskier surface: it rewrites rule code/params) | `self_modify_enabled` | **`false`** |
| **L2 modulation** | Continuous, bounded `RuleEffect` nudge + bias/constraint scaling | `l2.enabled` | **`true`** |

**Consequence:** `self_modify_enabled = false` does **not** stop L2 from modulating
`RuleEffect`. That is by design, not a bug — L2 performs small, reversible,
bounded adjustments with an explicit rollback path (`l2_rollback_effect` +
negative-consequence feedback), whereas L1 commits discrete rule patches. To
freeze *all* policy mutation, disable **both**: `self_modify_enabled = false` and
`set_l2_enabled(false)`. The benchmark baseline arm relies on exactly this:
`set_l2_enabled(false)` is what removes L2 `RuleEffect` nudges.

This semantics is locked by the test
`api::tests::l2_modulates_effect_even_when_self_modify_disabled`.

## Layers

| Layer | Role |
|-------|------|
| L0 | observe → predict → decide → execute → learn |
| L1 | distill → reflect → optional structured patch |
| L2 | meta metrics → soft bias / fixed-point heuristic |

## Persistence

```
state.json            # KernelSnapshot (schema versioned)
audit.jsonl           # append-only events
trajectory.jsonl      # optional step dump (replay source)
checkpoints/          # rollback snapshots
```

See `docs/API_0.5.md` for the public 0.5 surface.
