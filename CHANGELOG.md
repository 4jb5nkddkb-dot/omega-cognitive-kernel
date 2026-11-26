# Changelog

## [Unreleased]

### Correctness / verification
- Make prioritized case replay deterministic across kernels, threads, and repeated runs. Schema v6 persists each memory's replay cursor; legacy snapshots migrate without granting L2 permissions.
- Add opt-in spectral-control experiments, non-mutating L0/patch dry-runs, per-step signal and execution diagnostics, paired bootstrap intervals, and documented acceptance criteria. Spectral intervention remains disabled by default; the preregistered discovery and independent validation runs did not demonstrate improvement, so this route is stopped without further parameter tuning.
- Make benchmark reports byte-replayable with key-sorted aggregation/serialization and local replay cursors. Keep wall-clock patch metadata out of fixed-point control scoring.
- Enable bounded L2 interventions by default for new kernels and missing `[l2]` configuration while keeping L1 self-modification disabled. Legacy snapshots without an explicit L2 permission still migrate with L2 disabled.
- Document the schema v6 replay-cursor state, retain v5 migration coverage, and keep Python snapshot checks without changing the historical 0.6.0 schema-v2 release notes.
- Return path/line/byte-offset errors for malformed audit records in Rust and Python; retain incomplete trailing records for subsequent polling.
- Add per-seed paired L2 benchmark reports with descriptive 95% intervals so non-intervention differences cannot be mistaken for demonstrated L0 influence. Isolate task/tool-gate/audit-panel test state and benchmark sidecars in per-run directories.
- Attribute L2 feedback and benchmark rewards to the policy that actually changed the executed L0 decision, retain KEEP_L0 selection propensities, consume each committed snapshot once, and preserve queued policy state across restart without injecting it during snapshot restoration. Add per-expert applied/not-applied evidence; spectral cognitive-realm output remains observational and does not grant execution permission.
- Fix the Rust 1.74 Clippy gate, cover the active development branch in CI, and stop recursively deriving auto-commit subjects from previous commits.

## [0.6.0] — 2026-11-22

### Highlights
- **Generic decision pipeline**: `Kernel<O, D, E>` now supports custom `DecisionMaker` implementations in addition to observer and executor adapters. Added the `BanditKernel` alias and `Kernel::with_bandit_ucb(path)` for UCB-based bandit tasks.
- **L2 disabled by default**: `L2MetaLayer` starts with `enabled = false`. Metrics remain available for observability and controlled comparisons, but no biases, constraints, or `RuleEffect` are modified unless callers explicitly enable L2. The UCB treatment shows a small positive benchmark delta, while evidence across broader tasks and seed ranges remains necessary before changing the safe default.
- **Measurable nonstationary bandit delta**: the UCB-based treatment reports positive mean-reward and best-arm-rate deltas over the constrained-decision, L2-disabled baseline. This compares different decision policies, so it does not isolate the effect of L2 modulation or establish production effectiveness.
- **Schema v2 and fail-closed persistence**: safety and rollback state survives restarts, and durable audit, trajectory, and autosave failures propagate through `Kernel::step` instead of being silently discarded.

### Correctness / safety (P0)
- **Fail-closed persistence**: `Kernel::step` now propagates audit-append, trajectory-write, and autosave failures instead of silently ignoring them (`let _ = ...`). A step can no longer report success while the audit trail or saved state dropped the transition. In-process event hooks fire only after a successful audit append.
- **Pre-patch checkpoint decoupled from `PatchAccepted`**: a failure to save the optional pre-patch rollback checkpoint no longer suppresses the primary `PatchAccepted` audit event.
- **Schema v2** (`CURRENT_SCHEMA_VERSION = 2`): `KernelSnapshot` now persists three safety/restore fields that were previously lost on restart:
  - `self_modify_enabled` — enabling L1 then restarting no longer silently reverts to the safe default-off.
  - `steps_since_last_patch` — the L1 patch cooldown survives restart instead of resetting.
  - `l2_rollback_effect` — the pre-commit policy needed for negative-feedback rollback survives an L2 commit + restart.
  - v0/v1 snapshots migrate up automatically; the new fields default to the safe pre-v2 behavior (L1 disabled, warm cooldown, no rollback effect).
- **L2 default disabled**: `L2MetaLayer` now starts with `enabled = false`. The monitor still evaluates metrics for observability and baseline comparisons, but no biases, constraints, or `RuleEffect` are modified unless the caller explicitly enables L2 with `Kernel::set_l2_enabled(true)`. See the highlighted evidence-driven default above.

### Maintenance
- Honor explicit `--state` and configuration paths in `demo`, isolate the default demo's sidecar files, and add an end-to-end demo/status/audit regression test.
- Make quick-start Cargo commands select `omega-kernel` explicitly and make weekly report generation fail on runtime errors or inconsistent step counts.
- Save snapshots through a synced same-directory temporary file and atomic rename; failed replacement preserves the previous state and cleans up temporary files.
- Bound HTTP tool response reads before constructing previews and document the host's URL/SSRF policy responsibility.
- Expose benchmark action diversity, switching, and post-commit action/arm changes; report measured treatment deltas rather than treating commits as success.
- Align current source version, installation tags, and Python documentation; distinguish source versions from Git tags and registry availability.
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
- Document the two independent self-modification gates in `ARCHITECTURE.md`: `self_modify_enabled` gates **L1 patch commits only** (default `false`); L2 `RuleEffect` modulation is an independent channel governed by `l2.enabled` (now default `false`) and still runs when self-modify is off. Locked by `api::tests::l2_modulates_effect_even_when_self_modify_disabled`.
- `RELEASING.md` documents the PyPI build/test/publish pipeline and the one-time Trusted Publisher setup.
- Clarify version mapping in `README.md`: distinguish source package versions from Git tags and registry availability.
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
