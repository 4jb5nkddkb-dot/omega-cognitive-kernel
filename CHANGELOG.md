# Changelog

## [0.3.5] — 2026-11-05

### Strategy idle decay

- `Strategy` tracks `idle_steps`; absorb ages all strategies and resets the chosen one
- `StrategyMemory::decay_idle` multiplies idle weights by a factor (default threshold 5, factor 0.92)
- `MemoryFacade::update` applies decay every step so early lucky weights cannot lock the policy

### Learner ownership

- `Learner` owns update + decay path (`update` / `decay_only`)
- Unit tests for learner decay behavior

### Integration tests

- Patch cooldown: accepted patches must be at least `min_steps_between_patches` apart
- Constraints: tight `max_risk` discourages explore
- L2 soft-nudge path can fire and change `explore_bonus`

## [0.3.4] — 2026-11-05

### L2 soft RuleEffect modulation

- Fixed-point `Modulation` now carries `explore_bonus_delta` / `risk_tolerance_delta`
- `MetaModulator::apply_to_effect` merges deltas into current `RuleEffect` then sanitizes
- Orchestrator applies the nudge to `memory.cognitive_effect` so L0 sees it next step

### Constraints in DecisionMaker

- `max_risk` from constraints now influences scoring: tight risk penalizes explore, boosts stabilize/observe
- Action params include `max_risk` for observability

### CLI / status L2 surface

- `KernelStatus` exposes `in_fixed_point`, `dwell_steps`, `last_alignment_score`, `max_risk`, `explore_bonus`, `risk_tolerance`
- `run` / `status` / `demo` print fixed-point state and soft-nudge events

## [0.3.3] — 2026-11-05

### L1 hardening

- **Abstractor** now extracts error variance, action distribution, and a heuristic success rate
- **Reflector** enforces a **cooldown** (`min_steps_between_patches`, default 3)
- **VersionedStore** history is FIFO-capped at 64 entries

### L2 fixed-point lifecycle

- **MetaMonitor.consistency** is data-driven
- **EmergenceDetector** adds hysteresis/cooldown and soft exit
- **L2MetaLayer** tracks dwell steps (entry → dwell → exit)

## [0.3.2] — 2026-11-05

### RuleEffect expansion

- Added bounded fields: `explore_bonus`, `risk_tolerance`
- DecisionMaker uses both; PatchGenerator is strategy-aware

## [0.3.1] — 2026-11-03

- Packaging, MIT, release workflow, checkpoint FIFO

## [0.3.0] — 2026-11-03

- S3: audit log, checkpoints/rollback, bandit task shell

## [0.2.0] — 2026-11-03

- S2: persistence, Kernel API, CLI

## [0.1.0] — 2026-11-03

- S1: L0/L1/L2 skeleton, measurable RuleEffect consequence
