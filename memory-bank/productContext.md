# Product Context

## Product promise

The product is an auditable adaptive loop for agents and applications that need to change strategy online without retraining. Its operational promise is: observe, patch a structured policy surface, remember the consequence, measure the result, and roll back when the result is negative.

## User-facing surfaces implemented in code

A Rust integrator can call `Kernel::new`, `Kernel::open`, `Kernel::with_state_path`, or `Kernel::with_adapters_and_path`. The main operations are `step`, `run`, `status`, `save`, `load`, `checkpoint`, `rollback`, `audit_summary`, `audit_tail`, `replay_trajectory`, `set_event_hook`, `set_strategy_weight`, `set_l2_enabled`, and `observe_l2_consequence`.

The CLI exposes the same operational model through the `omega-kernel` binary, including demo, run, status/status JSON, audit, replay, and L2 benchmark flows. The optional GUI dashboard and audit panel consume persisted state/audit/replay data. Python exposes `Kernel`, `AuditLog`, and `CheckpointStore` through the PyO3 module `omega_cognitive_kernel_python`.

## Product behavior users must understand

There are two separate forms of adaptation. L1 accepts a discrete `CognitivePatch` only when `self_modify_enabled` is true. L2 can still modulate `RuleEffect` while L1 is off because `l2.enabled` is a separate switch. To freeze both mutation paths, callers must disable both switches.

The default configuration is deliberately operational rather than autonomous: autosave is on, pre-patch checkpointing is on, trajectory dumping is on, L1 self-modification is off, L2 is off, the L1 mean-error threshold is `0.30`, the L1 cooldown is `3` steps, and the decision `max_risk` constraint is `0.65`.

## Auditability and recovery experience

Every successful `step` emits a `Step` audit event and may emit fixed-point, L2 modulation, patch accepted/rejected, and checkpoint events. `event_hook` fires only after the corresponding audit append succeeds. `audit.jsonl` can be tailed or polled by byte offset, while `trajectory.jsonl` can be summarized offline without re-executing the world.

Checkpoints store complete snapshots beside the state file. A rollback restores the snapshot and records a `Rollback` event. State writes use a synced temporary file and atomic rename, and persistence errors are returned instead of being silently ignored.

## Licensing and boundary

The public core is MIT-licensed. The repository documentation keeps hosted multi-tenant services, managed policy packs, and enterprise control-plane concepts outside the default public core. These are product boundary statements, not implemented runtime capabilities in the current source tree.
