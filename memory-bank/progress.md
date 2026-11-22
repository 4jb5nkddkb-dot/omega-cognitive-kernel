# Progress

## Implemented in the current source tree

The core runtime is implemented as a generic observer/decision/executor kernel with a model-based predictor and constrained-decision default. `BanditKernel` and `Kernel::with_bandit_ucb(path)` provide a UCB decision-maker specialization. The L0 path observes, retrieves cases/strategies/CAS/effect context, predicts, decides under constraints, executes, computes uncertainty/action-based error, and updates memory.

L1 includes trajectory abstraction, reflection, a hard self-modification gate, bounded effect sanitization, sandbox validation, versioned active-module storage, accepted/rejected outcomes, and a patch cooldown. L2 includes metric monitoring, emergence detection, fixed-point scoring, bounded modulation of biases/constraints/effect, cooldowns, consequence feedback, defensive mode, and negative-consequence rollback.

The operational surface includes schema-versioned JSON snapshots, v0/v1-to-v2 migration, atomic save, append-only JSONL audit, byte-offset audit polling, trajectory JSONL replay, checkpoints, rollback, status JSON, event hooks, strategy-weight nudges, configurable state paths, TOML configuration, and optional GUI/async/network/wasm/no-std-oriented feature paths.

The repository also contains Python bindings exposing `Kernel`, `AuditLog`, and `CheckpointStore`, integration examples, portability scripts, and documentation for integration, dashboard operation, closed-loop behavior, API, troubleshooting, and release work.

## Release and roadmap state

The source and Python package versions are `0.6.0`. The changelog has a dated 0.6.0 section covering generic decision makers, fail-closed persistence, schema v2 safety restoration, atomic replacement, HTTP read bounds, benchmark reporting, Python packaging/CI, and documentation alignment. L1 and L2 are disabled by default. The recommended nonstationary benchmark's UCB treatment has a small positive task-performance delta, but differing decision policies prevent interpreting it as an L2-only effect.

The roadmap marks the 0.5.x schema/replay/events/strategy weights/dashboard/CLI work as shipped. Full multi-tenant control plane, workspace multi-kernel orchestration, and optional docs.rs polish remain deferred.

## Verification state

For the 0.6.0 release preparation on 2026-11-22, `cargo check --locked`, `cargo test --lib --locked` (93 passed), `cargo fmt --all -- --check`, and `python3 scripts/check-release-version.py v0.6.0` passed. Python binding `cargo check --locked` also passed with the workflow's `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` setting and with `PYO3_PYTHON` set to a local Python 3.13 interpreter. The default local Python 3.14 interpreter is newer than PyO3 0.23's standard maximum supported version. The recommended 20-seed nonstationary UCB treatment run produced `delta_mean_reward = +0.046875` and `delta_best_arm_rate = +0.072222`; its error-decrease rate did not improve. The treatment uses a different decision maker from the baseline, so this is not an L2-only ablation. Full `cargo test`, `--all-features`, Python tests, release build, and portability probes were not run locally; consult their CI results separately.

## Next update protocol

After a code change, update the smallest relevant Memory Bank file. Record exact tests and commands that ran, their outcome, and any unverified assumptions. If a change touches persistence schema, audit events, public API, feature flags, or safety gates, update both the relevant context document and the progress/verification record.
