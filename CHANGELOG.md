# Changelog

## [Unreleased]

### Correctness / safety
- Reject weight-history squashes that would overflow finite values, preserving the original payload on failure.

### Persistence / safety
- Bound each audit JSONL file to 64 MiB by default, with Rust and Python per-kernel overrides and a ToolGate override. Oversized appends now fail before writing, preserving all earlier records; existing oversized files remain readable, and audit failures propagate instead of being silently ignored.

### Testing
- Add `benchmark-l2-gain`, a preregistered four-scenario L1/L2 factorial matrix with independent discovery/validation seed sets, explicit execution and safety gates, deterministic replay tests, and CI smoke artifacts. The completed 960-run matrix did not demonstrate a reproducible L2 reward gain; no production control defaults changed.

### Reliability
- Bound trajectory JSONL growth to 64 MiB by default, with a per-kernel Rust/Python override and serialized local appends. Oversized appends fail closed without truncating history; existing logs remain replayable.
## [0.8.0] - 2026-11-28

### Added
- Cross-process persistence for external closed-loop sessions via `schema v6`-compatible `ExternalLoopState` on `KernelSnapshot`, serializable `CooldownTracker`, and structured `AuditEvent::{ExternalStep, ExternalRollback, ExternalStepDuplicate}` events (`#131`).
- `--no-save` flag on `omega-kernel step-external` for stateless dry-runs (`#131`).
- Real `L2MetaLayer` engine bridge (`OmegaOrchestrator::step_l2_from_telemetry`) populating `kl_prior_bias`, `in_fixed_point`, `lyapunov_potential`, `lyapunov_veto`, `alpha`, `beta`, and `selected_expert` in `L2PriorPayload` across both Rust and PyO3 bindings (`#133`).
- 7-mode closed-loop benchmark suite (`run_closed_loop_matrix_benchmark` and `omega-kernel benchmark-loops --scenario steady|shift|noisy --steps N [--json]`) (`#135`).
- Boundary hardening and long-horizon stress tests covering `CooldownTracker` saturation caps, out-of-order `StepTelemetry` bursts, and 180-step noisy external loops (`#137`).
- Manual GitHub Actions helpers `Auto Format (rustfmt)` (`#132`) and `Bump Release Version` (`#134`).

### Fixed
- Rejected non-finite (`NaN` / `±inf`) prediction errors and risk values in `CooldownTracker::can_patch` (`#137`).
  
## [0.7.1] - 2026-11-28

### Documentation & Packaging
- Aligned PyPI long description (`bindings/python/README.md`) with current `v0.7.x` bindings, documenting `ClosedLoopSession` and all 7 `LoopPreset` modes (`#128`).
- Unified package `description` and added `[project.urls]` (`Homepage`, `Repository`, `Changelog`, `Issues`) across both root `pyproject.toml` and `bindings/python/pyproject.toml` (`#128`).
- Synchronized `README.md`, `docs/PRODUCT.md`, `ARCHITECTURE.md`, and `RELEASING.md` release checklist (`#128`).

## [0.7.0] - 2026-11-27

### Added
- `LayerSwitches`, `LoopPreset` (all 7 non-empty `(L0, L1, L2)` closed-loop modes), `LoopMode`, `L1RuntimeGate`, `KernelConfig::with_loop_preset`, and `KernelConfig::effective_switches` for backward-compatible layer routing (`#121`, `#122`).
- External telemetry feedback contract (`StepTelemetry`, `ExternalOutcome`, `ControlDirective`, `RulePatchPayload`, `L2PriorPayload`, `ClosedLoopKernel`) and `ClosedLoopSession` supporting both internal `L0` and external `L0`-off closed loops (`#121`, `#122`).
- `no_std`-compatible `CooldownTracker` with exponential rollback backoff and improvement reset to prevent patch/rollback oscillation when `L2` is disabled (`#122`).
- End-to-end 7-mode closed-loop integration test matrix in `tests/closed_loop_matrix.rs` (`#123`).
- `omega-kernel` CLI global `--preset` flag and `step-external` subcommand (`#124`).
- Python (PyO3) `ClosedLoopSession` binding (`from_preset`, `switches_json`, `step_internal_json`, `step_external_json`) with regression tests in `test_python_bindings.py` (`#125`).

## [0.6.1] — 2026-11-27

### CI / release
- Replace the removed maturin `--universal2` flag with `--target universal2-apple-darwin` and install both macOS Rust targets in the PyPI release workflow. Preserve the complete Linux, Windows, and macOS wheel build, native install/test, and publication gates.

## [0.6.0] — 2026-11-27

### Highlights
- **Generic decision pipeline**: `Kernel<O, D, E>` now supports custom `DecisionMaker` implementations in addition to observer and executor adapters. Added the `BanditKernel` alias and `Kernel::with_bandit_ucb(path)` for UCB-based bandit tasks.
- **L2 enabled by default**: new kernels and missing `[l2]` configuration enable bounded interventions. Legacy snapshots without an explicit L2 permission migrate with L2 disabled. The UCB treatment shows a small positive benchmark delta, while evidence across broader tasks and seed ranges remains necessary before drawing broad effectiveness conclusions.
- **Measurable nonstationary bandit delta**: the UCB-based treatment reports positive mean-reward and best-arm-rate deltas over the constrained-decision, L2-disabled baseline. This compares different decision policies, so it does not isolate the effect of L2 modulation or establish production effectiveness.
- **Schema evolution and fail-closed persistence**: safety, rollback, and deterministic replay state survives restarts through schema v6; durable audit, trajectory, and autosave failures propagate through `Kernel::step` instead of being silently discarded.
- **`BodyBus` and supporting primitives**: add `no_std` sensor/actuator channels with unauthorized writes disabled, a hybrid world predictor, and weight export, verification, migration, and delta-history utilities.

### Correctness / safety
- **L1 safety controls**: add Shadow dry-run validation, L1/L2 write mutual-exclusion arbitration, and automatic rollback after worsening patch consequences. Return the restored `RuleEffect` in `StepResult.l1.rollback_effect` only for an actual rollback. L1 self-modification remains disabled by default; `CodeSandbox` performs static validation rather than executing patch code.
- Make async observer/executor adapters reusable across calls, propagate async logging and tool-audit failures, and bound HTTP observation bodies at 1 MiB for both sync and async readers.
- Record L1 rejection only for actual validation failures, preserve `PatchAccepted` when automatic checkpoints are disabled, and resolve CLI configuration/state paths before opening a snapshot.
- Fail closed on malformed restored arm/expert indices and corrupt checkpoint metadata; make effect sanitization non-finite-safe and zero-capacity case memories bounded.
- Make prioritized case replay deterministic across kernels, threads, and repeated runs. Schema v6 persists each memory's replay cursor; legacy snapshots migrate without granting L2 permissions.
- Add opt-in spectral-control experiments, non-mutating L0 dry-runs, per-step signal and execution diagnostics, paired bootstrap intervals, and documented acceptance criteria. Spectral intervention remains disabled by default; the preregistered discovery and independent validation runs did not demonstrate improvement, so this route is stopped without further parameter tuning.
- Make benchmark reports byte-replayable with key-sorted aggregation/serialization and local replay cursors. Keep wall-clock patch metadata out of fixed-point control scoring.
- Return path/line/byte-offset errors for malformed audit records in Rust and Python; retain incomplete trailing records for subsequent polling.
- Add per-seed paired L2 benchmark reports with descriptive 95% intervals so non-intervention differences cannot be mistaken for demonstrated L0 influence. Isolate task/tool-gate/audit-panel test state and benchmark sidecars in per-run directories.
- Attribute L2 feedback and benchmark rewards to the policy that actually changed the executed L0 decision, retain KEEP_L0 selection propensities, consume each committed snapshot once, and preserve queued policy state across restart without injecting it during snapshot restoration. Add per-expert applied/not-applied evidence; spectral cognitive-realm output remains observational and does not grant execution permission.
- **Fail-closed persistence**: `Kernel::step` now propagates audit-append, trajectory-write, and autosave failures instead of silently ignoring them (`let _ = ...`). A step can no longer report success while the audit trail or saved state dropped the transition. In-process event hooks fire only after a successful audit append.
- **Pre-patch checkpoint decoupled from `PatchAccepted`**: a failure to save the optional pre-patch rollback checkpoint no longer suppresses the primary `PatchAccepted` audit event.
- **Schema evolution**: schema v2 introduced persistence for L1 enablement, patch cooldown, and the pre-commit L2 rollback effect, with safe migration from v0/v1. The release now uses schema v6, adding deterministic case replay, resident L2 state, and safe legacy defaults.
- **L2 default enabled**: new kernels and missing `[l2]` configuration enable bounded `RuleEffect` modulation; callers can use `Kernel::set_l2_enabled(false)` for monitoring-only baselines. Legacy snapshots without an explicit L2 permission still migrate with L2 disabled.

### Privacy / persistence
- Add opt-in `[privacy].redact_user_content` for known user goals, task names, and checkpoint labels written by the kernel; retain legacy defaults and schema compatibility.
- Create new Unix persistence directories with owner-only `0700` and files with owner-only `0600`; document existing-file behavior and Windows ACL inheritance.
- Document persistence paths, default recording scope, redaction limits, and cleanup of historical append-only logs.

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
- Persist CLI rollback results and record manual checkpoints and rollbacks in the audit log; add end-to-end checkpoint/rollback integration coverage and map invalid Python snapshot JSON to `ValueError`.
- Test isolation: API tests that exercise audit/checkpoint sidecar files now use a unique temporary subdirectory so the shared `<parent>/audit.jsonl` and `<parent>/checkpoints/` paths cannot leak between tests.

### CI / release
- Add the `lychee` offline Markdown link-check workflow and run `cargo test --doc --locked` for README doctests in CI.
- Build and test Python wheels across CPython 3.10–3.15. `release-pypi.yml` is split into build → test → publish: macOS (universal2) wheels and an sdist are added, every wheel is installed and `pytest`-tested on its native platform/version before publication, and publication is gated on those tests passing (Trusted Publishing via the `release` environment).

### Python support
- Officially support CPython 3.10–3.15, retaining `requires-python >=3.10`; update package classifiers, user/release documentation, standard CI, native wheel tests, and Linux/Windows/macOS PyPI wheel build/test matrices.

### Documentation and examples
- Add `examples/l1_probe.rs` to exercise L1 Shadow zero-perturbation, controlled L1/L2 write arbitration, and the automatic-rollback window through the public API. The probe validates the actual arbitration flag, isolates immediate versus expired rollback feedback, and exits nonzero on failed checks.
- Synchronize Vault documentation with XChaCha20-Poly1305 v2 encryption, CSPRNG-generated 24-byte nonces, and read-only legacy-v1 migration.
- Add the `benchmark-l1l2` four-arm factorial experiment with paired L1/L2 main-effect, joint, and interaction contrasts, common environment randomness, raw/adjusted reward metrics, and independent seed validation. The completed matrix did not demonstrate reproducible positive gains; production gate defaults are unchanged.
- Document the two independent self-modification gates in `ARCHITECTURE.md`: `self_modify_enabled` gates **L1 patch commits only** (default `false`); L2 `RuleEffect` modulation is an independent channel governed by `l2.enabled` (default `true`) and still runs when self-modify is off. Locked by `api::tests::l2_modulates_effect_even_when_self_modify_disabled`.
- Align schema-v6 migration, executable source-installation guidance, and independent L1/L2 defaults with the current runtime. Document L2 `L2Policy` execution attribution separately from commits and no-op proposals.
- Regenerate spectral benchmark checksums for the pre-action dry-run implementation, distinguish historical evidence from current verification, and mark the incomplete component-ablation series explicitly.
- Keep the drop-inbox example state in a configurable persistent directory, resume it on later runs, and document loading that exact state file in the dashboard.
- Sync `README.md`, `docs/API_0.6.md`, `examples/README.md`, architecture, closed-loop, and release documentation with current commands, paths, APIs, safety defaults, and repository layout.
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
