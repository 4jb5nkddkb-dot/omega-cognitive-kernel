# Progress

## Implemented in the current source tree

The core runtime is implemented as a generic observer/decision/executor kernel with a model-based predictor and constrained-decision default. `BanditKernel` and `Kernel::with_bandit_ucb(path)` provide a UCB decision-maker specialization. The L0 path observes, retrieves cases/strategies/CAS/effect context, predicts, decides under constraints, executes, computes uncertainty/action-based error, and updates memory.

L1 includes trajectory abstraction, reflection, a hard self-modification gate, bounded effect sanitization, sandbox validation, versioned active-module storage, accepted/rejected outcomes, and a patch cooldown. L2 includes metric monitoring, emergence detection, fixed-point scoring, a resident fail-closed controller, discounted-Hedge/IPS learning, KL-prior policy snapshots, online Dayan residuals, bounded Lyapunov gating, and a small Meta-Cache.

The operational surface includes schema-v5 JSON snapshots with safe legacy migration, atomic save, append-only JSONL audit, byte-offset audit polling, trajectory replay with applied/pending policy observability, checkpoints, rollback, status JSON, event hooks, strategy-weight nudges, configurable state paths, `[bandit]`/`[l2]` TOML configuration, and optional GUI/async/network/wasm/no-std-oriented feature paths.

The repository also contains Python bindings exposing `Kernel`, `AuditLog`, and `CheckpointStore`, integration examples, portability scripts, and documentation for integration, dashboard operation, closed-loop behavior, API, troubleshooting, and release work.

## Release and roadmap state

L1 and L2 remain disabled by default. The paired UCB benchmark varies only L2 enablement. Its 2026-11-23 20-seed runs did not produce executable L2 commits; cache hits did not demonstrate warm recovery or a best-arm improvement.

The roadmap marks the 0.5.x schema/replay/events/strategy weights/dashboard/CLI work as shipped. Full multi-tenant control plane, workspace multi-kernel orchestration, and optional docs.rs polish remain deferred.

## Verification state

On 2026-11-23, `cargo test --locked` passed all 124 tests (120 library, 3 audit-panel, 1 CLI), and `cargo test --all-features` additionally passed the dashboard test. `cargo clippy --locked --all-targets -- -D warnings` and `bash scripts/test-no-std.sh` passed. Both wasm library targets compile; the WASI smoke script enables `std` for the host facade. With a temporary Wasmtime runtime, the smoke test passed snapshot restore and continued from step 4 to step 5. Three release benchmark commands each used 20 seeds beginning at `20261115`, 12 episodes × 12 steps, for `nonstationary`, `regime_shift`, and `cyclic_shift`; all completed and are documented in `docs/L2_BENCHMARK.md`. No executable L2 commits occurred, so warm recovery is unverified rather than claimed.

## Next update protocol

After a code change, update the smallest relevant Memory Bank file. Record exact tests and commands that ran, their outcome, and any unverified assumptions. If a change touches persistence schema, audit events, public API, feature flags, or safety gates, update both the relevant context document and the progress/verification record.
