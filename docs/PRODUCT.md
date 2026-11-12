# Product narrative (Omega Cognitive Kernel)

## One-liner

**Training-free, auditable online strategy adaptation** — inject observe/act adapters, patch bounded `RuleEffect` policies, checkpoint and roll back.

## Who it is for

- Teams that need **runtime policy adaptation** without retraining weights
- Systems that must **audit** why the policy changed and **undo** bad patches
- Integrators who already have sensors / files / HTTP status and want a small closed loop

## What ships today (v0.4.7)

| Capability | Surface |
|------------|---------|
| Kernel API | `Kernel`, `run` / `step` / status / checkpoint / rollback |
| Config | `omega.toml` via `KernelConfig` and CLI `--config` |
| Safety switch | `self_modify_enabled` (default **false**) |
| Trajectory | `dump_trajectory` → `trajectory.jsonl` |
| Adapters | `Observer` / `Executor`; File, Dir, HTTP, Logging, ToolGate |
| CLI | `omega-kernel`, `omega-audit-panel` |
| GUI | `omega-dashboard` (`--features gui`) |
| Evidence | Audit log, checkpoints, bandit metrics, weekly report Actions |

## Demo story (sellable)

**Drop-inbox loop** (`examples/drop_inbox_loop.rs`) and **Dashboard** operator UI.

```bash
cargo run --example drop_inbox_loop
cargo run --release --features gui --bin omega-dashboard
```

## Monetization angles (honest)

**Open core (recommended early)**

- Core kernel + adapters + CLI + dashboard: MIT / public
- Paid later: hosted multi-tenant audit store, managed policy packs, support SLA

**Not claimed**

- Not a full agent OS, not AGI, not a replacement for RL training stacks
- L2 fixed-point language is **heuristic**

## Next product steps

1. Keep 0.4.x freeze; ship operator UX (dashboard) and safety defaults
2. Optional crates.io when API churn stays low
3. Real caller-driven service/auth only when a concrete deployment needs it
