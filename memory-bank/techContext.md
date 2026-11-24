# Technical Context

## Toolchain and crate surface

The crate uses Rust 2021 with `rust-version = "1.74"`. `src/lib.rs` is `no_std`-capable through `#![cfg_attr(not(feature = "std"), no_std)]` and always imports `alloc`. Standard-library modules such as API, audit, checkpoint, config, persistence, and task are feature-gated.

The public crate re-exports `Kernel`, `KernelStatus`, `RunReport`, trajectory replay types, audit types, configuration types, `KernelSnapshot`, `OmegaOrchestrator`, core domain types, and adapter traits. The library generates process-local UUIDs from an atomic sequence; under non-`std`, time falls back to the Unix epoch string.

## Feature model

The default features are `std`, `ureq`, and `toml`. `gui` enables eframe/egui/rfd; `async` enables Tokio, async-std, futures, and reqwest; `wasm` enables getrandom JavaScript support; `msrv` provides the compatible standard subset; and `docsrs` enables documentation attributes. Dependency pins in `Cargo.toml` deliberately avoid Rust 2024-only transitive versions for the Rust 1.74 target.

## Module map

`src/api.rs` owns the product-facing generic `Kernel`, default construction, configuration application, step/run/status, snapshot conversion, save/load, checkpoint/rollback, event hooks, audit access, and trajectory replay. `src/orchestrator.rs` composes the three layers.

`src/types.rs` defines `Observation`, `Prediction`, `Intent`, `Action`, `Trajectory`, `RuleEffect`, `CognitivePatch`, `MetaMetrics`, and `SystemState`. `src/l0` defines `Observer`, `Predictor`, `DecisionMaker`, `Executor`, and `L0WorldLayer`. `src/l1` defines the abstraction/reflector/sandbox/store patch path. `src/l2` defines monitor, detector, fixed-point scorer, modulator, and consequence handling. `src/memory` defines case, strategy, CAS, and facade state.

## On-disk artifacts

The default state file is `.omega/state.json`. Audit and trajectory files are placed beside it as `audit.jsonl` and `trajectory.jsonl`; checkpoint snapshots are stored under a sibling `checkpoints/` directory. The state schema is `CURRENT_SCHEMA_VERSION = 5` and includes total steps, biases, constraints, CAS metrics, `RuleEffect`, cases, strategies, active modules, patch history, L1 state, the resident L2 controller, bounded Meta-Cache, online Dayan residual, validated pending/applied policy snapshots, and UCB explanations.

`KernelSnapshot::load_from` deserializes and migrates older versions through v5. Missing legacy L2 permissions default to `false`; malformed or noncommitted policy snapshots fail closed to `KEEP_L0`. A newer unsupported schema is rejected. `save_to` writes pretty JSON to a synced same-directory temporary file and atomically renames it.

## Configuration defaults

`KernelConfig` defaults to `.omega/state.json`, autosave/checkpoint/trajectory dumping enabled, error threshold `0.30`, minimum patch interval `3`, `max_risk = 0.65`, `self_modify_enabled = false`, and `l2.enabled = true`. Intervention policy defaults to exploration mode, maximum level `SuggestAlternative`, friction threshold `2`, and escalation auditing enabled; strict mode caps effective escalation at `RefuseToDecide` for higher requested levels.

## Verification entry points

The source contains unit tests in the implementation modules and integration tests in `tests/`. Relevant regression names cover L0/L1/L2 orchestration, patch cooldown, rule preference, fixed-point behavior, event hooks, schema round trips/migration, atomic replacement, audit polling, and configuration policy. The repository also provides `scripts/test-no-std.sh` and `scripts/test-wasm-runtime.sh`.

The Memory Bank initialization inspected source and documentation but did not run the full build/test matrix. Any future update must record actual command results in `progress.md` rather than inferring success from source inspection.
