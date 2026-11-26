# Project Brief: Omega Cognitive Kernel

## What the repository implements

Omega Cognitive Kernel is a Rust library and CLI runtime for online strategy adaptation without model-weight updates. The public `Kernel<O, D, E>` type injects an `Observer`, a `DecisionMaker`, and an `Executor`, while the predictor remains `ModelBasedPredictor`. The default concrete kernel uses `MultiModalObserver`, `ConstrainedDecisionMaker`, and `SandboxedExecutor`; `BanditKernel` substitutes the UCB decision maker.

The implementation changes a bounded `RuleEffect`, not executable source code. `RuleEffect` contains `error_damping`, action-name preferences, `min_uncertainty`, `explore_bonus`, and `risk_tolerance`. Its sanitizer clamps values and removes unknown action names; the allowed actions are `noop`, `observe`, `explore`, `exploit`, and `stabilize`.

## Runtime objective

For each high-level goal, the runtime should observe the world, retrieve relevant memory, predict, choose a constrained action, execute it, calculate prediction error, learn from the trajectory, optionally accept an L1 patch, and let L2 evaluate whether a meta-level modulation is justified. All meaningful transitions should be persisted or exposed through the audit stream.

```text
Kernel::step(goal)
  -> OmegaOrchestrator::run_once
     -> L0WorldLayer::step
        observe -> memory.retrieve -> predict -> decide -> execute
        -> compute_errors -> Learner::update
     -> L1CognitiveLayer::process
     -> L2MetaLayer::step
  -> audit events -> trajectory.jsonl -> autosave state.json
```

## Explicit safety boundaries

L1 is the discrete patch channel and is disabled by default through `self_modify_enabled = false`. L1 must distill a trajectory, reflect a candidate patch, sanitize its `RuleEffect`, validate it through `CodeSandbox`, and only then update the active effect and versioned store. A cooldown counter limits immediate repatching.

L2 is independent and enabled by default for new kernels. It monitors metrics even when disabled. A validated commit queues an `L2Policy` expert/KL prior for a subsequent L0 decision; it does not bypass L0 dwell/margin/safety checks. Negative consequence feedback can restore the saved pre-commit effect. Legacy snapshots without an explicit L2 permission remain disabled after migration.

## Repository structure

The main Rust implementation is under `src/`. `src/l0` contains observer, predictor, decision, and executor boundaries; `src/l1` contains abstraction, reflection, sandbox, and versioned patch storage; `src/l2` contains monitoring, emergence detection, fixed-point scoring, and modulation; `src/memory` contains cases, strategies, CAS state, and the `MemoryFacade`. `src/api.rs` is the product-facing persistence, audit, replay, checkpoint, rollback, configuration, and status surface.

The repository also includes Python bindings under `bindings/python`, examples under `examples/`, integration tests under `tests/`, portability probes under `portable/`, and operator/product documentation under `docs/`.

## Baseline captured from source

The working tree was clean before Memory Bank generation. The source package declares version `0.6.0`, Rust 2021, and MSRV `1.74`. `CURRENT_SCHEMA_VERSION` is `6`; the default state path is `.omega/state.json`.
