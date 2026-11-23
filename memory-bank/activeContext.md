# Active Context

## Source snapshot

This context reflects the `L2改进` source tree after PR #28 and its subsequent L2 changes. The crate and Python binding declare source version `0.6.0`, Rust 2021, MSRV `1.74`, and persistence schema `5`. The declared source version predates the latest controller changes and must be separated before publication.

## Current implementation focus

The 0.6.0 changelog and code cover generic decision makers and correctness/safety at the product step boundary. `Kernel<O, D, E>` accepts a `DecisionMaker`; `BanditKernel` and `with_bandit_ucb` support UCB-based bandit decisions. Persistence failures are fail-closed: audit append, trajectory write, and autosave errors return through `Kernel::step`. Event hooks run only after audit append. Atomic snapshot replacement preserves the previous file if replacement fails. Schema v2 persists L1 enablement, patch cooldown, and the L2 pre-commit effect across restarts; schema v5 adds UCB and resident-controller state.

The source also includes a complete L2 consequence loop: L2 commits only after score/cooldown/defensive checks, waits for feedback, and can restore the pre-commit `RuleEffect` after negative consequence. The status surface exposes fixed-point, cooldown, consequence, and defensive-mode state.

## Active code path to keep stable

For changes affecting behavior, start at `Kernel::step` in `src/api.rs`, follow `OmegaOrchestrator::run_once`, then inspect the relevant layer and persistence/audit code. The most compatibility-sensitive paths are `RuleEffect::sanitize`, L1's `self_modify_enabled` gate, L2's independent `enabled` gate, `KernelSnapshot::migrate`, and the ordering of audit/trajectory/autosave writes.

## Deferred product work

The roadmap still defers a full multi-tenant control plane, a workspace multi-kernel orchestration service, and possible docs.rs feature-flag polish. These are not present as runtime services in the current source.

## Open engineering questions

The current branch adds resident L2 state through schema v5, including controller/cache state and validated pending/applied policy snapshots. Current stabilization work keeps L1/L2 disabled by default, checks legacy migration, reports malformed audit records, adds per-seed benchmark evidence, and restores the MSRV/Python verification gates. Optional feature combinations, especially `async`, `gui`, `wasm`, and `msrv`, deserve continued CI coverage when dependencies change. Any schema or audit enum change requires a migration/compatibility decision before implementation.

## Maintenance rule

When active work changes, update this file with the exact module/path, behavior, blocker, and verification result. Keep claims grounded in source or executed tests; this file is not a substitute for the API or architecture documentation.
