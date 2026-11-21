# Active Context

## Source snapshot

This context was refreshed from the checked-out `main` source tree at commit `f7ae3de` (`创建 EXPERIMENTS.md`). The working tree was clean before the Memory Bank files were changed. The crate declares source version `0.5.25`, Rust 2021, MSRV `1.74`, and persistence schema `2`.

## Current implementation focus

The unreleased changelog and code are concentrated on correctness and safety around the product step boundary. Persistence failures are fail-closed: audit append, trajectory write, and autosave errors return through `Kernel::step`. Event hooks run only after audit append. Atomic snapshot replacement preserves the previous file if replacement fails. Schema v2 persists L1 enablement, patch cooldown, and the L2 pre-commit effect across restarts.

The source also includes a complete L2 consequence loop: L2 commits only after score/cooldown/defensive checks, waits for feedback, and can restore the pre-commit `RuleEffect` after negative consequence. The status surface exposes fixed-point, cooldown, consequence, and defensive-mode state.

## Active code path to keep stable

For changes affecting behavior, start at `Kernel::step` in `src/api.rs`, follow `OmegaOrchestrator::run_once`, then inspect the relevant layer and persistence/audit code. The most compatibility-sensitive paths are `RuleEffect::sanitize`, L1's `self_modify_enabled` gate, L2's independent `enabled` gate, `KernelSnapshot::migrate`, and the ordering of audit/trajectory/autosave writes.

## Deferred product work

The roadmap still defers a full multi-tenant control plane, a workspace multi-kernel orchestration service, and possible docs.rs feature-flag polish. These are not present as runtime services in the current source.

## Open engineering questions

The repository documents a source version of `0.5.25` while also documenting a newer repository tag `v0.5.30`; release/version alignment should remain explicit. Optional feature combinations, especially `async`, `gui`, `wasm`, and `msrv`, deserve continued CI coverage when dependencies change. Any schema or audit enum change requires a migration/compatibility decision before implementation.

## Maintenance rule

When active work changes, update this file with the exact module/path, behavior, blocker, and verification result. Keep claims grounded in source or executed tests; this file is not a substitute for the API or architecture documentation.
