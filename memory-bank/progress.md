# Progress

## Implemented in the current source tree

The core runtime is implemented as a generic observer/decision/executor kernel with a model-based predictor and constrained-decision default. `BanditKernel` and `Kernel::with_bandit_ucb(path)` provide a UCB decision-maker specialization. The L0 path observes, retrieves cases/strategies/CAS/effect context, predicts, decides under constraints, executes, computes uncertainty/action-based error, and updates memory.

L1 includes trajectory abstraction, reflection, a hard self-modification gate, bounded effect sanitization, sandbox validation, versioned active-module storage, accepted/rejected outcomes, and a patch cooldown. L2 includes metric monitoring, emergence detection, fixed-point scoring, a resident fail-closed controller, discounted-Hedge/IPS learning, KL-prior policy snapshots, online Dayan residuals, bounded Lyapunov gating, and a small Meta-Cache.

The operational surface includes schema-v6 JSON snapshots with safe legacy migration, atomic save, append-only JSONL audit, byte-offset audit polling, trajectory replay with applied/pending policy observability, checkpoints, rollback, status JSON, event hooks, strategy-weight nudges, configurable state paths, `[bandit]`/`[l2]` TOML configuration, and optional GUI/async/network/wasm/no-std-oriented feature paths.

The repository also contains Python bindings exposing `Kernel`, `AuditLog`, and `CheckpointStore`, integration examples, portability scripts, and documentation for integration, dashboard operation, closed-loop behavior, API, troubleshooting, and release work.

## Release and roadmap state

L1 remains disabled by default; new kernels enable L2 by default. The paired UCB benchmark varies only L2 enablement. A separate `benchmark-l1l2` command compares L0, L1-only, L2-only, and L1+L2 with common seeded environments, per-seed factorial contrasts, and raw/adjusted reward metrics. The 2026-11-26 preregistered four-scenario, two-seed-set matrix found no reproducible positive benefit. In validation `regime_shift`, L2-only and joint reward deltas were negative with Student-t 95% intervals entirely below zero; see `docs/EXPERIMENTS.md` for the complete results and scope.

Historical L2 results: the 2026-11-23 20-seed runs below produced no executable commits. The 2026-11-26 30-seed ordinary-L2 experiments did produce attributed interventions, but did not meet the preregistered reward criterion or reproduce the positive discovery estimate in validation. Neither result demonstrates an L2 benefit.

The roadmap marks the 0.5.x schema/replay/events/strategy weights/dashboard/CLI work as shipped. Full multi-tenant control plane, workspace multi-kernel orchestration, and optional docs.rs polish remain deferred.

## Verification state

### L1/L2 factorial benchmark, 2026-11-26

On `experiment/l1-l2-factorial`, added deterministic four-arm benchmark tests, gate isolation, per-seed interaction checks, raw/adjusted reward accounting, and L2 arbitration observability. Passed `cargo test --locked` (196 library tests plus CLI/doc targets), `cargo test --locked --all-features` (202 library tests plus GUI/CLI/doc targets), stable Clippy with warnings denied, formatting, `cargo test --locked --doc`, and Rust 1.74 `msrv` Clippy/tests (179 library tests plus applicable targets). Completed four two-seed preflight reports and the fixed eight-report 30-seed release matrix. Reports are in `/tmp/omega-l1-l2-factorial-f065e74/` with SHA-256 checksums; this temporary location is not a durable CI artifact.

### Comprehensive code review, 2026-11-26

Reviewed the runtime, safety/persistence, async/network adapters, CLI, and Python binding paths from the `bc3b636` main baseline. Fixed one-shot async adapter consumption, swallowed async/tool-audit errors, unbounded HTTP observation reads, incorrect L1 rejection/acceptance auditing, CLI config-path loading, unsafe restored indices, and silently corrupt checkpoint metadata. Schema v6 and public audit variants remain unchanged; disabled L1 no longer creates `PatchRejected` events, and disabling automatic checkpoints no longer suppresses `PatchAccepted`.

Passed: `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets --all-features -- -D warnings`; `cargo test --locked --all-targets` (176 library tests); `cargo test --locked --all-features --all-targets` (180 library tests plus binary/CLI targets); `cargo +1.74.0 clippy --locked --all-targets --no-default-features --features msrv -- -D warnings`; `cargo +1.74.0 test --locked --all-targets --no-default-features --features msrv` (165 library tests plus binary/CLI targets); `bash scripts/test-no-std.sh`; WASI smoke with installed Wasmtime (restore and continue from step 4 to 5); Python 3.13/maturin with 12 pytest tests. Benchmark replay determinism also passed. Local execution used Python 3.13 because pinned PyO3 0.23 does not support the system Python 3.14.

### Documentation audit, 2026-11-26

Baseline `main` at `9b5e13a` includes schema v6 and automatic L1 rollback on worsening error. `cargo test --locked` passed 175 tests (170 library, 3 audit-panel, 2 CLI); `cargo test --locked --all-features` passed 176 tests with the dashboard test. The eight ordinary-L2 reports and eight spectral reports were regenerated in release mode. Ordinary results match `docs/EXPERIMENTS.md`; spectral reward outcomes match, but the corrected pre-action dry-run counts are 19 candidate changes and 9 selected changes for discovery `regime_shift`. Local Python and Rust 1.74 checks were not run during this documentation pass; the corresponding CI jobs are separate evidence.

### Historical verification records

The records below belong to their dated branches and source snapshots. They are not current test totals or substitutes for rerunning checks on the current tree.

### L2 default-on integration, 2026-11-24

The `L2-improvement` branch incorporates `origin/main` at `c3553cf`. Conflict resolution retains the upstream controller fixes and enables L2 consistently for new kernels, TOML defaults, Python status, and new snapshots. Missing legacy L2 permissions remain disabled. CI and no-std workflows use the current branch name. Passed: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked` (129 tests), Python 3.13/maturin with 11 pytest tests, 9 hook tests, and all 3 skill-pack validations. Python 3.14 is not supported by the pinned PyO3 0.23.5, so verification used the available Python 3.13 interpreter.

### Stabilization update, 2026-11-23

The active `L2-improvement` tree has schema-v5 documentation and Python coverage, legacy migration checks, explicit malformed-audit errors shared by Rust/Python, per-seed paired L2 reports with descriptive 95% intervals, isolated task/tool-gate/benchmark sidecars, and non-recursive auto-commit subjects. It incorporates the latest `origin/main` controller fixes. L1 remains disabled by default; new kernels and missing `[l2]` configuration enable L2. Legacy snapshots without an explicit L2 permission remain disabled.

Executed checks passed: `cargo test --workspace --all-targets` (128 tests), `cargo test --all-features` (129 including dashboard), `cargo +1.74.0 test --locked --all-targets --no-default-features --features msrv` (118 tests), Clippy with `-D warnings` on stable and Rust 1.74, `bash scripts/test-no-std.sh`, and the WASI smoke test using the locally available temporary Wasmtime runtime. Python 3.13/maturin passed all 11 pytest integration tests; the hook suite passed 9 tests and the skill pack validator passed. Eight workflow YAML files parsed successfully.

The final nonstationary release benchmark retained the earlier aggregate `delta_mean_reward = 0.000694444444444331` and `delta_best_arm_rate = 0.0010416666666666075`. All 20 paired runs had zero commits and `treatment_influenced_l0 = false`. Only seed `20261120` had a nonzero reward delta (`0.01388888888888884`), so the aggregate is not evidence of L2 policy influence. This is a mechanism diagnostic, not an effectiveness result.

### Earlier source baseline

On 2026-11-23, `cargo test --locked` passed all 124 tests (120 library, 3 audit-panel, 1 CLI), and `cargo test --all-features` additionally passed the dashboard test. `cargo clippy --locked --all-targets -- -D warnings` and `bash scripts/test-no-std.sh` passed. Both wasm library targets compile; the WASI smoke script enables `std` for the host facade. With a temporary Wasmtime runtime, the smoke test passed snapshot restore and continued from step 4 to step 5. Three release benchmark commands each used 20 seeds beginning at `20261115`, 12 episodes × 12 steps, for `nonstationary`, `regime_shift`, and `cyclic_shift`; all completed and are documented in `docs/L2_BENCHMARK.md`. No executable L2 commits occurred, so warm recovery is unverified rather than claimed.

## Next update protocol

After a code change, update the smallest relevant Memory Bank file. Record exact tests and commands that ran, their outcome, and any unverified assumptions. If a change touches persistence schema, audit events, public API, feature flags, or safety gates, update both the relevant context document and the progress/verification record.
