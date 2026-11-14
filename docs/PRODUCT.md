# Product narrative (Omega Cognitive Kernel)

## One-liner

**Training-free, auditable online strategy adaptation** — inject observe/act adapters, patch bounded `RuleEffect` policies, checkpoint and roll back.

## Who it is for

- Teams that need **runtime policy adaptation** without retraining weights
- Systems that must **audit** why the policy changed and **undo** bad patches
- Integrators who already have sensors / files / HTTP status and want a small closed loop

## What ships today (v0.5.6 on crates.io)

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

**Published:** [crates.io/crates/omega-cognitive-kernel](https://crates.io/crates/omega-cognitive-kernel) `0.5.6`

## Current productizable assets (2026-11-14 screening)

| Asset | Repo / Status | Notes |
|-------|---------------|-------|
| Rust kernel + CLI + Dashboard | `omega-cognitive-kernel` (public, crates.io) | Primary open-core surface. Dashboard already exists (egui). |
| Python reference implementation | `omega-core-grok` (private) | File Mission demo, Boundary, recent workflow hardening. Good internal reference. |
| Next-line placeholder | `omega-core-next` (public, empty) | Candidate for commercial control-plane or major version line. |
| Supporting crates | `file-seal`, `tat-tvam-asi` | Niche utilities; not core monetization path. |

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

## Next product steps (prioritized)

1. **Dashboard polish & demo readiness** — deepen operator UX, ensure examples are copy-paste friendly for integrators.
2. **Adoption path** — keep 0.5.x additive; improve INTEGRATING.md and examples coverage.
3. **omega-core-next decision** — either seed as commercial control-plane sketch or archive until needed.
4. Real caller-driven service/auth only when a concrete deployment needs it.
5. Keep the open-core boundary clear: new monetizable surface goes into the commercial layer, not the public core by default.

See also [PRODUCTIZATION_NOTES.md](./PRODUCTIZATION_NOTES.md) for the 2026-11-14 screening details.
