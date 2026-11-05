# Changelog

## [0.3.6] — 2026-11-05

### Learner path ownership

- `L0WorldLayer` holds a `Learner` and uses `learner.update` after every step (case/strategy/CAS + idle decay)
- `MemoryFacade::update` is the raw absorb only; decay is owned by Learner
- `L1CognitiveLayer` also carries a `Learner` for future post-patch hooks
- API / orchestrator constructors wire `Learner::default()`

### Tests

- Fixed-point exit path: after entry, degraded metrics / long dwell can leave the fixed point

## [0.3.5] — 2026-11-05

### Strategy idle decay

- `Strategy` tracks `idle_steps`; absorb ages all strategies and resets the chosen one
- `StrategyMemory::decay_idle` multiplies idle weights by a factor
- Learner owns update + decay path

### Integration tests

- Patch cooldown spacing
- Tight `max_risk` discourages explore
- L2 soft-nudge path can change `explore_bonus`

## [0.3.4] — 2026-11-05

### L2 soft RuleEffect modulation

- Fixed-point carries `explore_bonus_delta` / `risk_tolerance_delta`
- Orchestrator merges sanitized nudge into `cognitive_effect`

### Constraints in DecisionMaker

- `max_risk` influences scoring

### CLI / status L2 surface

- `in_fixed_point`, `dwell_steps`, `last_alignment_score`, `max_risk`

## [0.3.3] — 2026-11-05

- L1: richer Abstractor, Reflector cooldown, VersionedStore bound 64
- L2: data-driven consistency, hysteresis/exit, dwell tracking

## [0.3.2] — 2026-11-05

- RuleEffect `explore_bonus` / `risk_tolerance`; strategy-aware patches

## [0.3.1] — 2026-11-03

- Packaging, MIT, release workflow, checkpoint FIFO

## [0.3.0] — 2026-11-03

- S3: audit, checkpoints/rollback, bandit task

## [0.2.0] — 2026-11-03

- S2: persistence, Kernel API, CLI

## [0.1.0] — 2026-11-03

- S1: L0/L1/L2 skeleton
