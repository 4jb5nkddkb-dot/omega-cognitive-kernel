# Progress

## Implemented in the current source tree

The core runtime is implemented as a generic observer/executor kernel with fixed predictor/decision defaults. The L0 path observes, retrieves cases/strategies/CAS/effect context, predicts, decides under constraints, executes, computes uncertainty/action-based error, and updates memory.

L1 includes trajectory abstraction, reflection, a hard self-modification gate, bounded effect sanitization, sandbox validation, versioned active-module storage, accepted/rejected outcomes, and a patch cooldown. L2 includes metric monitoring, emergence detection, fixed-point scoring, bounded modulation of biases/constraints/effect, cooldowns, consequence feedback, defensive mode, and negative-consequence rollback.

The operational surface includes schema-versioned JSON snapshots, v0/v1-to-v2 migration, atomic save, append-only JSONL audit, byte-offset audit polling, trajectory JSONL replay, checkpoints, rollback, status JSON, event hooks, strategy-weight nudges, configurable state paths, TOML configuration, and optional GUI/async/network/wasm/no-std-oriented feature paths.

The repository also contains Python bindings exposing `Kernel`, `AuditLog`, and `CheckpointStore`, integration examples, portability scripts, and documentation for integration, dashboard operation, closed-loop behavior, API, troubleshooting, and release work.

## Release and roadmap state

The source package version is `0.5.25`. The changelog has an `Unreleased` section covering fail-closed persistence, schema v2 safety restoration, atomic replacement, HTTP read bounds, benchmark reporting, Python packaging/CI, and documentation alignment. Project docs identify `v0.5.30` as the newest repository tag.

The roadmap marks the 0.5.x schema/replay/events/strategy weights/dashboard/CLI work as shipped. Full multi-tenant control plane, workspace multi-kernel orchestration, and optional docs.rs polish remain deferred.

## Verification state

For this Memory Bank refresh, the code and documentation were inspected directly. No full `cargo test`, `cargo test --all-features`, release build, Python test run, no-std probe, or WASM probe was executed. The commands in `techContext.md` are verification entry points, not claimed results.

## Next update protocol

After a code change, update the smallest relevant Memory Bank file. Record exact tests and commands that ran, their outcome, and any unverified assumptions. If a change touches persistence schema, audit events, public API, feature flags, or safety gates, update both the relevant context document and the progress/verification record.
