# Product narrative (Omega Cognitive Kernel)

## One-liner

**Training-free, auditable online strategy adaptation** — inject observe/act adapters, patch bounded `RuleEffect` policies, checkpoint and roll back.

## Who it is for

- Teams that need **runtime policy adaptation** without retraining weights
- Systems that must **audit** why the policy changed and **undo** bad patches
- Integrators who already have sensors / files / HTTP status and want a small closed loop

## What ships today (v0.3.9)

| Capability | Surface |
|------------|---------|
| Kernel API | `Kernel`, `run` / `step` / status / checkpoint / rollback |
| Config | `omega.toml` via `KernelConfig` and CLI `--config` |
| Adapters | `Observer` / `Executor`; File, Dir snapshot, HTTP, Logging |
| Safety | Action whitelist, RuleEffect bounds, L1 cooldown, L2 soft-only |
| Evidence | Audit log, checkpoints, bandit task metrics |

## Demo story (sellable)

**Drop-inbox loop** (`examples/drop_inbox_loop.rs`):

1. External process drops files into an inbox directory
2. `DirSnapshotObserver` feeds structured observations
3. Kernel may accept a bounded `RuleEffect` patch under rising error
4. `LoggingExecutor` records actions to a side-channel log
5. Checkpoints / audit remain available for review

```bash
cargo run --example drop_inbox_loop
```

## Monetization angles (honest)

**Open core (recommended early)**

- Core kernel + adapters + CLI: MIT / public
- Paid later: hosted dashboard, multi-tenant audit store, managed policy packs, support SLA

**Not claimed**

- Not a full agent OS, not AGI, not a replacement for RL training stacks
- L2 fixed-point language is **heuristic**

## Next product steps

1. One vertical reference integration (inbox / HTTP health / queue depth)
2. Freeze public API notes toward 0.4
3. Optional crates.io publish when API churn slows
4. Thin dashboard reading `audit.jsonl` + checkpoints (commercial candidate)
