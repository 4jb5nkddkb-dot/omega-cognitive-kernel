# Changelog

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
