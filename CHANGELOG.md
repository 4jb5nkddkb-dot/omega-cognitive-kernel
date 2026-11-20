# Changelog

## [Unreleased]

### Maintenance
- Align current source version, installation tags, and Python documentation; distinguish source versions from newer repository tags.
- Use one dynamic Python package version sourced from `bindings/python/Cargo.toml`.
- Replace the print-based Python test script with isolated pytest integration tests and add Python tests to CI.
- Remove tracked backup/build artifacts and ignore future backup files.
- Fix the audit-panel test module syntax that prevented the default Rust test suite from compiling.
- Record manual checkpoints and rollbacks in the audit log; map invalid Python snapshot JSON to `ValueError`.


## [0.5.7] — 2026-11-17

### Async I/O (feature = "async")
- New `AsyncObserver` / `AsyncExecutor` traits with native tokio async I/O
- `AsyncObserverAdapter` / `AsyncExecutorAdapter` to wrap sync adapters on thread pool
- Concrete async adapters: `AsyncDirSnapshotObserver`, `AsyncFileObserver`, `AsyncHttpObserver` (reqwest/ureq), `AsyncLoggingExecutor`

### CI / Infrastructure
- **L2 Baseline Regression job**: nonstationary bandit benchmark (5 seeds) with JSON artifact upload for trend tracking
- Fixed `rfd` backend for Linux CI: `xdg-portal` + `tokio` (pure Rust, no GTK3 system deps)

### L1
- `PatchGenerator` now uses `high_error_streak` more aggressively (extra damping on long streaks) and includes streak + recent mean error in the audit `code` comment and `expected_effect` string.

### Audit / L2
- Added `AuditEvent::FixedPointEntered` and `FixedPointExited` (additive, tagged enum). Summary counters updated.
- `Kernel::step` now emits these events when L2 enters or exits the fixed-point regime.

## [0.5.6] — 2026-11-14

### Packaging
- `package.include` now ships `examples/**/*` (clears crates.io dry-run warnings)
- crates.io dry-run verified; intentional first publish target

No API or schema change vs 0.5.5.

## [0.5.5] — 2026-11-14

### CLI
- `status --json` — machine-readable `KernelStatus` for scripts and integrators

### Examples / docs
- `audit_http_shim` `GET /status` (read-only)
- `event_hook_poll` example; INTEGRATING / API docs aligned
- `NOTICE` + open-core posture; crates.io checklist in RELEASING.md / CRATES_IO.md

## [0.5.4] — 2026-11-14

### CLI
- `strategy-weight <name> <weight>` operator nudge
- `audit-poll --offset N` incremental audit via `poll_after`

### Dashboard
- Audit **Follow** / Poll once / Reset cursor (live stream)

## [0.5.3] — 2026-11-14

### Dashboard
- Strategy weight operator nudge (`+` / `−` / reset to 1.0)
- Multi-workspace switcher (open / new / recent state.json instances)

## [0.5.2] — 2026-11-14

### Events
- `KernelEvent`, `poll_after` / `byte_len`, `set_event_hook`

### Strategies
- `set_strategy_weight`

## [0.5.1] — 2026-11-14
- Dashboard Replay tab; required GUI release artifacts

## [0.5.0] — 2026-11-14
- Schema migrate + trajectory replay

## Earlier
- See tags 0.4.x–0.5.x
