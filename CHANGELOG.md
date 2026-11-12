# Changelog

## [0.4.8] — 2026-11-12

### Case memory: prioritized experience replay

- Priority ∝ `|error_mean|^α` (α=0.6); zero-error floor `ε`
- Buffer full → **evict lowest priority** (no longer FIFO)
- `retrieve` samples without replacement ∝ priority
- `retrieve_with_is_weights` for optional importance-sampling weights + mild priority decay
- Legacy snapshots: missing `priority` field backfilled on restore
- Capacity remains **256** by default

## [0.4.7] — 2026-11-12

### Fixes

- `omega-dashboard`: match-arm type error in audit UI — GUI builds on CI
- Version strings aligned across Cargo.toml / README / docs

## [0.4.6] — 2026-11-12

### Desktop dashboard

- Optional feature `gui` + binary `omega-dashboard` (egui)

## [0.4.5] — 2026-11-12 (no public tag; folded into 0.4.6+)

- `self_modify_enabled`, `dump_trajectory`, audit-panel binary, weekly report

## [0.4.4] — 2026-11-10

- Audit HTTP shim demo

## [0.4.0] — 2026-11-09

- Integrator API freeze for 0.4.x

## Earlier

- See prior tags (0.3.x …)
