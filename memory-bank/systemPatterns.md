# System Patterns

## Generic adapter pipeline

`OmegaOrchestrator<O, P, D, E>` owns `L0WorldLayer<O, P, D, E>`, `L1CognitiveLayer`, and `L2MetaLayer`, plus mutable bias/constraint maps and a step counter. The product `Kernel<O, D, E>` fixes `P` to `ModelBasedPredictor`, defaults `D` to `ConstrainedDecisionMaker`, and allows custom observer, decision, and executor implementations. `BanditKernel` uses `BanditUcbDecisionMaker`.

## One runtime step

`L0WorldLayer::step` performs this exact sequence: `observer.observe()`; `memory.retrieve(goal, 5)`; `predictor.predict(observations, memory_context)`; `decision.decide(intent, predictions, constraints, memory_context)`; `executor.execute(actions)`; error calculation; and `Learner::update`. Prediction error is based on uncertainty plus a small action-quality term, clamped to `[0.05, 1.0]`; `stabilize` and `exploit` reduce the term while `explore` increases it.

Before L1 processes the new trajectory, the orchestrator checks an accepted patch's pending consequence against the pre-step CAS error. A worsening beyond the L1 threshold restores the previous effect and notifies L2. It then passes the trajectory to L1, builds a `SystemState` from active modules and CAS metrics, and calls L2. L2 queues an immutable `L2Policy` for the next L0 decision; the orchestrator records the policy actually attributed to the current action separately.

## L1 patch pattern

L1 increments `steps_since_last_patch`, distills the trajectory, and hard-stops with reason `self_modify_enabled=false` when the gate is off. When enabled, it builds context from CAS average error, strategy weights, and recent case errors. A reflector may create a `CognitivePatch`; the effect is sanitized, validated by `CodeSandbox`, committed to `VersionedStore`, installed as `memory.cognitive_effect`, and the cooldown resets. Failed validation returns a rejection reason without changing the active effect.

The patch's `code` and metadata are descriptive/auditable; the behavior-changing object is the bounded `RuleEffect`.

## L2 fixed-point and consequence pattern

L2 always evaluates metrics. When enabled, `EmergenceDetector` can trigger fixed-point scoring; `OmegaDayanFixedPoint` produces a score, alignment, and breakdown. A candidate only commits when its score reaches `commit_threshold` (default `0.82`), cooldown is zero, defensive mode is off, and consequence-stability conditions permit it. The immutable committed `L2Policy` supplies an expert prior and beta to the next L0 decision; L0 still applies dwell, margin, and safety checks. L2 retains rollback/consequence state and waits for feedback. A proposal or cache hit is not evidence that L0 applied an intervention.

`observe_consequence` computes `0.60 * reward_delta - 0.25 * error_delta - 0.15 * risk_delta`. A sufficiently negative result clears fixed-point state, extends cooldown, increases the negative-consequence streak, and can enter defensive mode; `take_rollback_effect` returns the saved effect for restoration. Positive feedback increments stable consequence steps and can recover defensive mode.

## Persistence and event ordering

`Kernel::step` takes an optional pre-step snapshot, runs the orchestrator, appends the `Step` event, appends L2 transition events, appends the trajectory record, records patch/checkpoint events, and autosaves. Audit append, trajectory write, and autosave errors propagate. The optional pre-patch checkpoint is best effort, but a successful checkpoint gets its own audit event; a checkpoint failure does not erase the fact that a patch was accepted.

The append-only audit event enum covers steps, accepted/rejected patches, checkpoints, rollbacks, task episodes, tool allow/deny, fixed-point transitions, and L2 modulation proposals/commits.

## Memory model

`MemoryFacade` composes `CaseMemory`, `StrategyMemory`, `CasState`, and the active `RuleEffect`. Retrieval returns a JSON context containing cases, strategies, CAS state, and cognitive effect. Learning adds the trajectory to cases, absorbs it into strategy memory, and updates CAS; external reward is clamped to `[0.0, 1.0]` before entering strategy memory.
