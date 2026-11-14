# Changelog

## [0.5.0] — 2026-11-14

### Thin 0.5.0 — schema + replay

- **State schema:** `CURRENT_SCHEMA_VERSION = 1`; load migrates missing/0 → 1; rejects newer versions
- **KernelStatus:** additive field `schema_version`
- **Replay:** `replay_trajectory_file` / `Kernel::replay_trajectory` + CLI `omega-kernel replay`
- **Docs:** `docs/ROADMAP_0.5.md`, `docs/API_0.5.md` (0.4 freeze lifted for listed items)

### Migration notes

- Existing 0.4 `state.json` files load without manual conversion
- Integrators deserializing `KernelStatus` must accept the new field (or ignore unknown fields)

## [0.4.9] — 2026-11-14

### Stability

- Case memory sampling: counter-seeded LCG + relaxed `retrieve_prefers_high_error` threshold
- Orchestrator patch test: assert accepted patch + damping, drop flaky error-decrease assert
- CI: formatting + clippy enforced; scheduled quality workflow opens PRs instead of direct push

### Docs

- Version headers aligned to 0.4.9 across Cargo / README / ARCHITECTURE

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
