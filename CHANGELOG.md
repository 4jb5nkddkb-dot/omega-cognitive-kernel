# Changelog

## [0.3.3] — 2026-11-05

### L1 hardening

- **Abstractor** now extracts error variance, action distribution, and a heuristic success rate (in addition to mean error).
- **Reflector** uses the richer features and enforces a **cooldown** (`min_steps_between_patches`, default 3) so patches cannot thrash on noisy trajectories.
- **VersionedStore** history is FIFO-capped at 64 entries.
- Unit tests added for Abstractor, Reflector, and store bounds.

### L2 fixed-point lifecycle

- **MetaMonitor.consistency** is data-driven (inverse of prediction error + generative/code presence) instead of a constant.
- **EmergenceDetector** adds hysteresis/cooldown and a soft **exit** signal when metrics degrade.
- **L2MetaLayer** tracks dwell steps, supports entry → dwell → exit, and periodically re-applies mild modulation while inside the fixed point.
- **OmegaDayanFixedPoint** alignment score enriched with keyword signals, code self-ref tokens, and sustained low-error maturity.

### Docs

- ARCHITECTURE.md updated for cooldown, store bound, and fixed-point lifecycle.

## [0.3.2] — 2026-11-05

### RuleEffect expansion

- Added bounded fields: `explore_bonus`, `risk_tolerance`
- Sanitize + validate_bounds cover the new fields
- DecisionMaker uses both (explore score bias + adaptive high-uncertainty threshold)
- Predictor / status / CLI surface the new values

### Smarter patches

- `PatchGenerator` now accepts `PatchContext` (strategy weights, recent mean error, high-error streak)
- L1 builds context from Case + Strategy memory before reflecting
- High-error regimes produce stabilize-biased, low-tolerance effects; low-error regimes allow mild explore when strategies are still weak

### Task / observability

- Bandit is more clearly non-stationary (best arm can flip twice)
- `BanditReport` includes `final_explore_bonus`, `final_risk_tolerance`, `patches_accepted`
- CLI `task` / `status` / `demo` / `run` print the new metrics

### Docs

- Added `ARCHITECTURE.md` (L0/L1/L2 data flow, RuleEffect contract, safety table)

## [0.3.1] — 2026-11-03

### Packaging

- MIT `LICENSE`
- Cargo metadata for install/publish (`repository`, `keywords`, `categories`)
- Release workflow on `v*` tags (Linux binary artifact)
- Install docs: `cargo install --git …`

### Hardening (included since 0.3.x line)

- External task reward → strategy memory
- `RuleEffect::sanitize` / `validate_bounds`
- Checkpoint FIFO (`max_keep = 32`)
- Seeded bandit + deterministic tests

## [0.3.0] — 2026-11-03

- S3: audit log, checkpoints/rollback, bandit task shell
- Product narrative: training-free `RuleEffect` patch runtime

## [0.2.0] — 2026-11-03

- S2: persistence, Kernel API, CLI (`demo` / `run` / `status` / `reset`)

## [0.1.0] — 2026-11-03

- S1: L0/L1/L2 skeleton, measurable `RuleEffect` consequence
