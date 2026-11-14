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
- L1 patch commit gated by **`self_modify_enabled`** (default **false**).
- On-disk state is **schema-versioned** (`CURRENT_SCHEMA_VERSION`).
- Trajectory can be **replayed offline** (CLI + dashboard) without world re-execution.

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
