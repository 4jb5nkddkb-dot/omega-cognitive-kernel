# Changelog

## [Unreleased]

### Correctness / safety (P0)
- **Fail-closed persistence**: `Kernel::step` now propagates audit-append, trajectory-write, and autosave failures instead of silently ignoring them (`let _ = ...`). A step can no longer report success while the audit trail or saved state dropped the transition. In-process event hooks fire only after a successful audit append.
- **Pre-patch checkpoint decoupled from `PatchAccepted`**: a failure to save the optional pre-patch rollback checkpoint no longer suppresses the primary `PatchAccepted` audit event.
- **Schema v2** (`CURRENT_SCHEMA_VERSION = 2`): `KernelSnapshot` now persists three safety/restore fields that were previously lost on restart:
  - `self_modify_enabled` — enabling L1 then restarting no longer silently reverts to the safe default-off.
  - `steps_since_last_patch` — the L1 patch cooldown survives restart instead of resetting.
  - `l2_rollback_effect` — the pre-commit policy needed for negative-feedback rollback survives an L2 commit + restart.
  - v0/v1 snapshots migrate up automatically; the new fields default to the safe pre-v2 behavior (L1 disabled, warm cooldown, no rollback effect).

### Maintenance
- Align current source version, installation tags, and Python documentation; distinguish source versions from newer repository tags.
- Use one dynamic Python package version sourced from `bindings/python/Cargo.toml`.
- Replace the print-based Python test script with isolated pytest integration tests and add Python tests to CI.
- Remove tracked backup/build artifacts and ignore future backup files.
- Fix the audit-panel test module syntax that prevented the default Rust test suite from compiling.
- Record manual checkpoints and rollbacks in the audit log; map invalid Python snapshot JSON to `ValueError`.
- Test isolation: API tests that exercise audit/checkpoint sidecar files now use a unique temporary subdirectory so the shared `<parent>/audit.jsonl` and `<parent>/checkpoints/` paths cannot leak between tests.

### CI / release
- CI `python` job now builds and tests the manylinux2014 wheel across CPython 3.10–3.13 (was 3.13 only), matching the versions the release publishes.
- `release-pypi.yml` is split into build → test → publish: macOS (universal2) wheels and an sdist are added, every wheel is installed and `pytest`-tested on its native platform/version before publication, and publication is gated on those tests passing (Trusted Publishing via the `release` environment).

### Documentation
- Document the two independent self-modification gates in `ARCHITECTURE.md`: `self_modify_enabled` gates **L1 patch commits only** (default `false`); L2 `RuleEffect` modulation is an independent channel governed by `l2.enabled` (default `true`) and still runs when self-modify is off. Locked by `api::tests::l2_modulates_effect_even_when_self_modify_disabled`.
- `RELEASING.md` documents the PyPI build/test/publish pipeline and the one-time Trusted Publisher setup.
- Clarify version mapping in `README.md`: distinguish between current source version (0.5.25) and newest repository tag (v0.5.30), which may include unreleased changes.
- Add `docs/TROUBLESHOOTING.md` with common issues and solutions for installation, runtime, configuration, performance, and Python-specific problems.



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
