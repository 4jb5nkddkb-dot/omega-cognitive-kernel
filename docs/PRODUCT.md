# Product narrative (Omega Cognitive Kernel)

## One-liner

**Training-free, auditable online strategy adaptation** — inject observe/act adapters, patch bounded `RuleEffect` policies, checkpoint and roll back.

## Who it is for

- Teams that need **runtime policy adaptation** without retraining weights
- Systems that must **audit** why the policy changed and **undo** bad patches
- Integrators who already have sensors / files / HTTP status and want a small closed loop

## What ships today (v0.5.4+)

| Capability | Surface |
|------------|---------|
| Kernel API | `Kernel`, `run` / `step` / status / checkpoint / rollback |
| Config | `omega.toml` via `KernelConfig` and CLI `--config` |
| Safety switch | `self_modify_enabled` (default **false**) |
| Trajectory | `dump_trajectory` → `trajectory.jsonl` + `replay` |
| Events | `KernelEvent` / `AuditEvent`; `AuditLog::poll_after`; in-process event hook |
| Strategies | Operator weight nudge (`set_strategy_weight`) |
| Adapters | `Observer` / `Executor`; File, Dir, HTTP, Logging, ToolGate |
| CLI | `omega-kernel` (incl. `strategy-weight`, `audit-poll`), `omega-audit-panel` |
| GUI | `omega-dashboard` (`--features gui`) — status / run / audit live follow / strategies / workspaces / replay |
| Evidence | Audit log, checkpoints, bandit metrics, weekly report Actions |

## Demo story (sellable)

**Drop-inbox loop** and **Dashboard** operator UI.

```bash
cargo run --example drop_inbox_loop
cargo run --release --features gui --bin omega-dashboard
```

## Monetization & licensing posture (honest)

**Open-core (current and recommended early path)**

- Core kernel + adapters + CLI + dashboard: **MIT**, public, low-friction integration
- Historical releases through v0.5.x remain MIT (irreversible)
- Brand / trademark protection via [NOTICE](../NOTICE): the code is free to use; the "Omega" name and commercial positioning are not automatically granted

**Intended commercial layer (future, separate terms)**

- Hosted multi-tenant audit store / control plane
- Managed policy packs and industry strategy packs
- Enterprise approval workflows, compliance reporting
- Support / SLA

These capabilities are deliberately **not** in the public core so they can later be offered under commercial or source-available terms without breaking existing MIT users.

**Not claimed**

- Not a full agent OS, not AGI, not a replacement for RL training stacks
- L2 fixed-point language is **heuristic**

## IP hygiene (ongoing)

1. **NOTICE** file present — distinguishes code license from trademark and commercial rights
2. Core stays permissively licensed while adoption is the priority
3. Before accepting non-trivial external contributions that touch the core, a CLA should be introduced so future dual-licensing or commercial exception remains possible
4. From a future major line (e.g. 0.6+), core may move MIT → Apache 2.0 for explicit patent grant if copyright ownership is clean

## Next product steps

1. Keep 0.5.x additive; deepen operator UX and event integration examples
2. Optional crates.io when API churn stays low
3. Real caller-driven service/auth only when a concrete deployment needs it
4. Keep the open-core boundary clear: new monetizable surface goes into the commercial layer, not the public core by default
