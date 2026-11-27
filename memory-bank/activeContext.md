# Active Context

## Source snapshot

This context reflects the `main` source tree after PR #42 and the documentation corrections prepared on `docs/repo-audit-fixes`. The crate and Python binding declare source version `0.6.0`, Rust 2021, MSRV `1.74`, and persistence schema `6`. Source-package versions and registry/tag availability are tracked separately.

## Current implementation focus

The 0.6.0 changelog and code cover generic decision makers and correctness/safety at the product step boundary. `Kernel<O, D, E>` accepts a `DecisionMaker`; `BanditKernel` and `with_bandit_ucb` support UCB-based bandit decisions. Persistence failures are fail-closed: audit append, trajectory write, and autosave errors return through `Kernel::step`. Event hooks run only after audit append. Atomic snapshot replacement preserves the previous file if replacement fails. Schema v2 persists L1 enablement, patch cooldown, and the L2 pre-commit effect across restarts; schema v6 additionally retains controller state and a per-instance case-replay cursor.

L1 restores its previous `RuleEffect` when an accepted patch worsens error by more than the configured threshold on the next observation. L2 queues an immutable `L2Policy` for a subsequent L0 decision, tracks its actual execution attribution, and retains separate consequence/rollback state. The status surface exposes fixed-point, cooldown, consequence, and defensive-mode state.

## Active code path to keep stable

For changes affecting behavior, start at `Kernel::step` in `src/api.rs`, follow `OmegaOrchestrator::run_once`, then inspect the relevant layer and persistence/audit code. The most compatibility-sensitive paths are `RuleEffect::sanitize`, L1's `self_modify_enabled` gate, L2's independent `enabled` gate, `KernelSnapshot::migrate`, and the ordering of audit/trajectory/autosave writes.

## Deferred product work

The roadmap still defers a full multi-tenant control plane, a workspace multi-kernel orchestration service, and possible docs.rs feature-flag polish. These are not present as runtime services in the current source.

The current branch `experiment/l1-l2-factorial` adds the standalone `benchmark-l1l2` fixture in `src/task.rs` and `src/main.rs`. It compares L0, L1-only, L2-only, and L1+L2 without changing production gate defaults or state schema. `StepResult::l1_blocked_by_l2` and `BanditReport::mean_adjusted_reward` expose mechanism and learning-feedback accounting. The completed 30-seed discovery/validation matrix is recorded in `docs/EXPERIMENTS.md`; it does not establish a reproducible L1/L2 benefit. Raw JSON artifacts remain outside the repository.

## Open engineering questions

The current tree retains resident L2 state through schema v6, including controller/cache state, validated pending/applied policy snapshots, and a per-instance case-replay cursor. New kernels and missing `[l2]` configuration enable L2 by default; legacy snapshots without an explicit L2 permission remain disabled. L1 self-modification remains disabled by default. Current documentation distinguishes the `L2Policy` intervention path from L1's discrete `RuleEffect` patches. Optional feature combinations, especially `async`, `gui`, `wasm`, and `msrv`, deserve continued CI coverage when dependencies change. Any schema or audit enum change requires a migration/compatibility decision before implementation.

## Maintenance rule

When active work changes, update this file with the exact module/path, behavior, blocker, and verification result. Keep claims grounded in source or executed tests; this file is not a substitute for the API or architecture documentation.
