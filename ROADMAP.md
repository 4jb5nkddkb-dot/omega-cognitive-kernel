# Omega Cognitive Kernel Roadmap

This roadmap describes the next development sequence for Omega Cognitive Kernel based on the current source tree, Memory Bank, changelog, benchmarks, and the existing 0.5 roadmap.

The project is currently a research prototype. The source package declares version `0.6.1`, and the current on-disk schema is `6`. The 0.6.0 source line contains the core L0/L1/L2 loop, structured `RuleEffect` adaptation, audit/replay, checkpoints/rollback, async adapters, Python bindings, generic decision makers, and an optional dashboard.

## Planning principles

1. **Safety before autonomy.** Keep L1 self-modification disabled by default, preserve the independent L2 gate, and require bounded effects, explicit consequence feedback, and rollback paths for new mutation behavior.
2. **Evidence before product claims.** Do not describe L2 modulation as an improvement until reproducible benchmarks demonstrate better or safer outcomes than the disabled-L2 baseline.
3. **Compatibility is a feature.** Treat public Rust APIs, Python wrappers, `KernelSnapshot`, `AuditEvent`, JSONL formats, feature flags, and the core Rust package's 1.74 MSRV as compatibility surfaces. The Python binding has a separate Rust build MSRV.
4. **Operational visibility is mandatory.** New behavior must be inspectable through status, audit, trajectory, metrics, or an equivalent deterministic test artifact.
5. **Keep the open core focused.** Hosted multi-tenancy, managed policy packs, and enterprise control-plane features should remain separately scoped rather than silently expanding the MIT runtime.

## Current baseline

### Already shipped in the source tree

The current implementation provides a generic `Kernel<O, D, E>` over observer, decision maker, and executor adapters, a default model-based predictor, case/strategy/CAS memory, L1 patch reflection and sandbox validation, L2 fixed-point scoring and bounded modulation, consequence feedback and defensive mode, schema-versioned snapshots, atomic saves, append-only audit events, byte-offset audit polling, trajectory replay, checkpoints, rollback, status JSON, strategy-weight nudges, TOML configuration, async adapters, Python bindings, examples, and a GUI dashboard.

### Language and build compatibility

- **Core Rust:** The root `Cargo.toml` declares Rust 1.74 as its MSRV, and CI tests the core package with Rust 1.74 using the `msrv` feature ([CI workflow](.github/workflows/ci.yml)).
- **Python:** The package declares CPython 3.10–3.15 support, and CI builds and tests a wheel for each version ([package metadata](pyproject.toml), [Python CI](.github/workflows/ci.yml)). The release workflow defines Linux, Windows, and macOS wheel build and install-test jobs for the same version range ([PyPI workflow](.github/workflows/release-pypi.yml)).
- **Python binding build:** The binding package separately declares Rust 1.83 and uses PyO3 0.29.3; this build minimum is not the core Rust MSRV ([binding manifest](bindings/python/Cargo.toml), [binding MSRV CI](.github/workflows/ci.yml)).

### Known baseline gaps

The paired benchmark now distinguishes commits, counterfactual candidate/selection changes, and formally attributed L0 interventions. The 2026-11-26 ordinary-L2 experiments produced executable commits, but failed the preregistered reward criterion and did not reproduce the positive discovery estimate in validation. Spectral-control experiments also failed to demonstrate benefit and remain opt-in. Evidence across broader tasks and seed ranges is still needed. Full feature-matrix verification and release reproducibility remain ongoing work. Multi-tenant control-plane and workspace orchestration are intentionally deferred.

## Phase 0 — Stabilize and release the current line

**Priority:** P0

**Status:** In progress. The source manifests declare version `0.6.1`; `v0.6.0` has an immutable GitHub Release, and the 0.6.1 release restores the complete PyPI wheel pipeline.

**Objective:** Make the current 0.6.x behavior easy to verify, document, and release before adding new policy mechanisms.

### Work items

- Publish and verify the declared source line before presenting version-specific registry or tag installation commands.
- Keep schema v6 migration, L1/L2 independent gates, `L2Policy` execution attribution, current status fields, and fail-closed persistence semantics synchronized with source.
- Preserve the CI verification matrix for default, `--all-features`, Python, async, GUI, no-std, and WASM targets.
- Archive benchmark JSON with its full implementation commit, command, diagnostic-definition version, and stdout checksum; avoid unverifiable local-only evidence.
- Extend deterministic fault injection and resource-limit coverage as described in Phase 1.

### Exit criteria

- A clean checkout can run the documented Rust and Python verification commands.
- The declared source version, Python version, package metadata, release notes, and installation examples agree.
- API/architecture docs describe the actual schema v6 and current L1/L2 behavior.
- L2 benchmark artifacts can be regenerated and compared across commits with their diagnostic-definition version recorded.
- No release is presented as verified when a required matrix entry was skipped.

## Phase 1 — Reliability, determinism, and observability

**Priority:** P0/P1

**Objective:** Make a kernel run diagnosable and reproducible enough for real integration experiments.

### Work items

- Add explicit resource limits for case memory, trajectory size, audit growth, HTTP/tool response sizes, and checkpoint retention.
- Define stable versioning rules for `AuditEvent` and trajectory JSONL records, including additive-field handling and malformed-record behavior.
- Add snapshot integrity metadata or an equivalent corruption-detection strategy without weakening atomic replacement.
- Separate deterministic simulation fixtures from stochastic/default demos so regression tests can assert exact outcomes where appropriate.
- Expand `KernelStatus` and audit summaries only through documented additive changes; include persistence health and last-error visibility where safe.
- Add fault-injection tests for audit, trajectory, autosave, checkpoint, rollback, and interrupted-write paths.
- Provide a structured diagnostics/export command for a sanitized state summary that does not expose sensitive observations by default.

### Exit criteria

- Bounded resource behavior is documented and tested.
- A partial write, malformed line, or failed sidecar operation has a defined recovery behavior.
- A failing integration can identify the last successful step, audit offset, snapshot version, active gates, and checkpoint state.
- Deterministic fixtures cover L0 decisions, L1 patch acceptance/rejection, L2 commit/rollback, and restart restoration.

## Phase 2 — Adapter and language integration maturity

**Priority:** P1

**Objective:** Make the runtime practical to embed without requiring users to understand internal modules.

### Work items

- Publish a small, stable adapter contract for observer/executor implementations, including timeout, cancellation, error, and idempotency guidance.
- Give sync and async adapters equivalent documented semantics and feature-gated compile coverage.
- Improve Python parity for status, audit streaming, consequence feedback, L2 controls, and custom integration boundaries where safe; keep arbitrary Python callbacks out of the core until their safety model is specified.
- Add integration examples for restart recovery, custom state paths, checkpoint/rollback, audit polling, and bounded tool execution.
- Add a versioned configuration reference generated or checked against `KernelConfig` defaults.
- Provide a migration guide for users moving from older snapshots and older 0.5.x APIs.

### Exit criteria

- A Rust integrator can implement a real observer/executor from the integration guide and understand failure semantics.
- Python documentation accurately lists the supported surface and limitations.
- Examples cover the principal safe deployment patterns and run in CI where practical.
- Configuration and migration docs are validated against source defaults.

## Phase 3 — Evaluation-driven adaptation improvements

**Priority:** P1

**Objective:** Improve L1/L2 policy quality only when experiments show a measurable safety or performance benefit.

### Work items

- Extend the fixed-seed scenarios beyond the existing stable, nonstationary, degraded, fixed-point, regime-shift, and cyclic-shift fixtures into noisy, adversarial, and high-risk environments.
- Evaluate each new controller hypothesis with fixed L1/L2 ablations, paired seeds, confidence intervals, and preregistered acceptance criteria; do not resume stopped parameter-tuning routes without a new hypothesis.
- Track reward, prediction error, risk, action diversity, switching cost, patch frequency, rollback frequency, and defensive-mode duration.
- Retain bounded pre-action counterfactual analysis and its distinction from actual execution attribution; never substitute model self-evaluation for external reward.
- Keep experimental heuristics behind explicit configuration and label them as experimental until promoted.

### Exit criteria

- Benchmark reports distinguish proposal, commit, consequence, rollback, and final outcome.
- A change to L1/L2 includes a before/after benchmark artifact and regression tests.
- The project can state when a modulation strategy is beneficial, neutral, or harmful rather than treating every commit as success.

## Phase 4 — Operator and multi-workspace maturity

**Priority:** P1/P2

**Objective:** Improve local operations while keeping the public core single-process and auditable.

### Work items

- Add dashboard views for persistence health, event offsets, checkpoint retention, L1/L2 gate state, consequence feedback, and rollback history.
- Define workspace metadata and safe switching semantics for multiple local kernel state files.
- Add export/import and redaction workflows for audit, trajectory, snapshot, and benchmark artifacts.
- Add a local supervisor or orchestration library only after workspace lifecycle, locking, concurrency, and failure semantics are specified.
- Document concurrency expectations: single-writer state, multiple-reader audit/replay, file rotation, and process ownership.

### Exit criteria

- An operator can inspect, pause, checkpoint, roll back, export, and resume a local workspace without editing JSON by hand.
- Concurrent reader/writer behavior is covered by tests or explicitly constrained.
- Workspace operations do not silently mix state, audit, trajectory, or checkpoints between kernel instances.

## Phase 5 — Separate control-plane exploration

**Priority:** P2 / outside the default core

**Objective:** Explore multi-tenant and hosted orchestration without compromising the MIT runtime's simple safety model.

### Work items

- Write a separate architecture proposal for tenant isolation, authentication, authorization, quotas, encrypted state, key management, audit retention, and incident response.
- Define whether the control plane belongs in a separate repository/package or an explicitly optional layer.
- Specify a stable kernel service protocol based on versioned commands/events rather than exposing internal `kernel.inner.*` layout.
- Threat-model networked observers, remote executors, tool allow-lists, policy packs, and cross-tenant data paths.
- Prototype only after the local single-kernel persistence and compatibility contracts are stable.

### Exit criteria

- The control-plane design has a threat model, data ownership model, migration strategy, and operational SLOs.
- Hosted functionality is clearly separated from the public MIT core and does not change local default safety behavior.
- No multi-tenant feature is considered production-ready without isolation and recovery tests.

## Cross-cutting backlog

| Area | Next useful increment | Success signal |
|---|---|---|
| Versioning | Single source/release narrative | README, Cargo, Python, tags, and release docs agree |
| Persistence | Fault injection and corruption recovery | Failed writes never appear as successful steps |
| Audit | Event/trajectory compatibility contract | Consumers tolerate additive fields and can resume polling |
| Safety | Explicit L1/L2 policy matrix | Gate behavior is visible in tests, status, and docs |
| Evaluation | Fixed-seed baseline artifacts | L2 claims are backed by reproducible comparisons |
| Integrations | Adapter timeout/error contract | Custom adapters can handle failure without internal knowledge |
| Python | Documented supported parity | Python API docs match the compiled wrapper |
| Operations | Workspace export/redaction | Operators can inspect and share safe diagnostics |
| Governance | Security/PR/release documentation | Contributors can validate and report changes consistently |

## Explicit non-goals for the next line

The next release line should not add arbitrary code execution as a policy mechanism, silently enable L1 self-modification, claim learning improvements without benchmark evidence, break old snapshots without migration, expose internal `kernel.inner.*` layout as a stable API, or introduce hosted multi-tenancy into the default local crate.

## How to update this roadmap

Every completed item should link to its implementation, tests, documentation, and benchmark evidence where applicable. When a roadmap item changes public API, snapshot schema, audit events, safety defaults, or feature flags, update `CHANGELOG.md`, the relevant API/architecture document, and the corresponding Memory Bank files. Record deferred work with a reason rather than leaving ambiguous checkboxes.
