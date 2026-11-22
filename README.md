# Omega Cognitive Kernel

**Training-free online strategy adaptation for auditable agents and adaptive applications.**

Omega Cognitive Kernel changes a bounded, structured policy surface instead of updating model weights or executing arbitrary code. It observes a task, predicts and chooses an action, learns from the result, optionally applies a reversible policy patch, measures the consequence, and records the complete transition.

> **Patch. Remember. Measure. Roll back.**

```text
World (L0) -> Reflect & patch (L1) -> Meta-modulate (L2) -> bias next L0
```

## Why Omega?

Adaptive systems often require retraining/fine-tuning or allow unrestricted code changes that are difficult to inspect and reverse. Omega takes a narrower approach: online adaptation is expressed as bounded `RuleEffect` data, validated before use, persisted as cognitive state, and exposed through audit and replay surfaces.

The current repository is a **research prototype**. The source package declares version `0.5.25`; project documentation identifies `v0.5.30` as the newest repository tag, which may contain changes beyond the current source-version declaration.

## Core concepts

### L0: world interaction

`L0WorldLayer` connects an observer, predictor, decision maker, executor, memory facade, and learner. Each step follows this sequence:

```text
observe
  -> retrieve cases, strategies, CAS state, and current RuleEffect
  -> predict
  -> decide under constraints
  -> execute
  -> compute prediction error
  -> update case/strategy/CAS memory
```

The default implementation uses `MultiModalObserver`, `ModelBasedPredictor`, `ConstrainedDecisionMaker`, and `SandboxedExecutor`. Integrators can inject custom observer and executor implementations through `Kernel<O, E>`.

### L1: discrete cognitive patches

L1 distills a trajectory and reflects a candidate `CognitivePatch`. When `self_modify_enabled` is enabled, the candidate is sanitized, validated by `CodeSandbox`, committed to the versioned store, and installed as the active `RuleEffect`. A cooldown prevents immediate repeated patching.

L1 is disabled by default. The patch metadata may contain descriptive code and expected-effect text, but the behavior-changing surface is the structured `RuleEffect`, not executable patch code.

### L2: bounded meta-modulation

L2 evaluates meta-metrics and can detect a fixed-point regime. When its alignment score, cooldown, defensive-mode, and consequence-stability checks allow a commit, it applies bounded changes to biases, constraints, and `RuleEffect`. L2 stores the pre-commit effect and accepts later reward/error/risk feedback through `observe_l2_consequence`.

L2 is an independent channel. **Both L1 and L2 are disabled by default.** To freeze all policy mutation, keep both gates off. Enable L2 explicitly with `set_l2_enabled(true)` only after reproducible benchmarks show a positive gain over the disabled baseline.

## Installation

### Use the Rust library

```bash
cargo add omega-cognitive-kernel
```

Or add the dependency manually:

```toml
[dependencies]
omega-cognitive-kernel = "0.5.25"
```

### Install the CLI from the repository

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel \\
  --tag v0.5.25 --locked
```

Install the optional dashboard with the `gui` feature:

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel \\
  --tag v0.5.25 --locked --features gui --bin omega-dashboard
```

Check the [Releases](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/releases) page for published binaries and the [crates.io package](https://crates.io/crates/omega-cognitive-kernel) for the latest published library version.

### Python bindings

```bash
python -m pip install omega-cognitive-kernel
```

The local binding module exposes `Kernel`, `AuditLog`, and `CheckpointStore` through PyO3. See [`bindings/python/README.md`](bindings/python/README.md) for development and testing instructions.

## Quick start

The following commands run the default CLI from a checkout. The demo uses an isolated temporary state directory; normal runtime commands use `.omega/state.json` unless an explicit state path is supplied.

```bash
# Run an isolated demonstration.
cargo run --release --bin omega-kernel -- demo

# Run 20 steps against the default state path.
cargo run --release --bin omega-kernel -- run --steps 20

# Inspect human-readable or machine-readable status.
cargo run --release --bin omega-kernel -- status
cargo run --release --bin omega-kernel -- status --json

# Inspect the append-only audit log.
cargo run --release --bin omega-kernel -- audit --summary

# Replay the last 15 trajectory rows without re-running the world.
cargo run --release --bin omega-kernel -- replay --tail 15

# Run the L2 benchmark and emit JSON.
cargo run --release --bin omega-kernel -- benchmark-l2 --runs 20 --json

# Start the optional operator dashboard.
cargo run --release --features gui --bin omega-dashboard
```

## Rust API

The default kernel persists state, audit events, and trajectory data under the state path. `Kernel::open(None)` resumes an existing state file when present or creates a new kernel otherwise.

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut kernel = Kernel::open(None)?;

    let report = kernel.run("maintain coherent self-model", 10)?;
    println!(
        "error {:.3} -> {:.3}; decreased={}",
        report.early_avg_err,
        report.final_avg_err,
        report.error_decreased
    );

    let replay = kernel.replay_trajectory(10)?;
    println!(
        "trajectory rows={}, accepted patches={}",
        replay.rows,
        replay.patches
    );

    kernel.save()?;
    Ok(())
}
```

Useful product-facing methods include:

| Method | Purpose |
|---|---|
| `step` / `run` | Execute one or multiple goal-driven cycles |
| `status` | Return operational metrics and safety state |
| `save` / `load` | Persist or restore the schema-versioned snapshot |
| `checkpoint` / `rollback` | Create or restore a full recovery snapshot |
| `audit_summary` / `audit_tail` | Inspect append-only audit events |
| `replay_trajectory` | Summarize trajectory JSONL without world re-execution |
| `set_event_hook` | Receive events after durable audit append |
| `set_l2_enabled` | Enable or disable L2 interventions |
| `observe_l2_consequence` | Feed reward/error/risk feedback to L2 |
| `set_strategy_weight` | Apply an operator strategy-weight nudge |

For custom world integration, implement the public `Observer` and `Executor` traits and construct the kernel with `Kernel::with_adapters` or `Kernel::with_adapters_and_path`.

## Configuration and safety defaults

`KernelConfig` can be loaded from TOML with `Kernel::from_config_path`. Its important defaults are:

| Setting | Default | Meaning |
|---|---:|---|
| `state_path` | `.omega/state.json` | Snapshot location |
| `autosave` | `true` | Save after successful steps |
| `auto_checkpoint_on_patch` | `true` | Capture pre-step state for patch recovery |
| `dump_trajectory` | `true` | Append a trajectory row for each step |
| `error_threshold` | `0.30` | L1 mean-error threshold |
| `min_steps_between_patches` | `3` | L1 patch cooldown |
| `max_risk` | `0.65` | L0 decision constraint |
| `self_modify_enabled` | `false` | L1 discrete patch gate |
| `l2.enabled` | `false` | L2 modulation gate (enable only after evidence) |

Example configuration:

```toml
state_path = ".omega/agent-state.json"
autosave = true
auto_checkpoint_on_patch = true
dump_trajectory = true
error_threshold = 0.30
min_steps_between_patches = 3
max_risk = 0.65
self_modify_enabled = false
```

## Persistence, audit, and replay

Given a state path such as `.omega/state.json`, the runtime stores:

```text
.omega/
├── state.json          # schema-versioned KernelSnapshot
├── audit.jsonl         # append-only AuditRecord stream
├── trajectory.jsonl    # optional per-step replay source
└── checkpoints/        # full rollback snapshots
```

The current on-disk schema is version `2`. It persists total steps, biases, constraints, CAS metrics, cases, strategies, active modules, patch history, L1 safety state, L2 detector/fixed-point state, and the pre-commit L2 rollback effect. Older v0/v1 snapshots migrate forward with safe defaults; newer unsupported schema versions are rejected.

State writes serialize to a same-directory temporary file, call `sync_all`, and atomically rename the file. Audit append, trajectory writes, and autosave failures are returned to the caller instead of being silently ignored.

The audit stream includes `Step`, `PatchAccepted`, `PatchRejected`, `Checkpoint`, `Rollback`, `TaskEpisode`, tool allow/deny events, fixed-point transitions, and L2 modulation proposal/commit events. `set_event_hook` is invoked only after the related audit record has been appended successfully.

Trajectory replay reads JSONL and reports row count, accepted patch count, first/last average error, action counts, and a selectable tail. It does not re-execute the world.

## Repository layout

```text
src/
├── api.rs             # public Kernel API, persistence, audit, replay, status
├── orchestrator.rs    # L0/L1/L2 composition and one-cycle execution
├── types.rs           # Observation, Action, Trajectory, RuleEffect, patches
├── l0/                # observers, predictors, decisions, executors
├── l1/                # abstraction, reflection, sandbox, versioned store
├── l2/                # metrics, emergence, fixed-point, modulation
├── memory/            # cases, strategies, CAS, MemoryFacade
├── persist.rs         # KernelSnapshot, migration, atomic save
├── audit.rs           # append-only events and polling
└── safety/            # default decision constraints

bindings/python/       # PyO3 bindings
examples/              # integration and adapter examples
tests/                 # integration tests
portable/              # no-std and WASM probes
docs/                  # API, integration, dashboard, portability, roadmap
memory-bank/           # persistent project context for contributors/agents
```

## Development and verification

The crate targets Rust `1.74` or newer and uses Rust 2021. Common checks are:

```bash
cargo test
cargo test --all-features
bash scripts/test-no-std.sh
bash scripts/test-wasm-runtime.sh
```

Relevant behavior is covered by unit and integration tests for orchestration, rule preferences, patch cooldown, fixed-point transitions, event hooks, schema migration, atomic replacement, audit polling, and intervention policy. The repository also includes Python integration tests and release/portability workflows.

Before changing behavior, start at `Kernel::step` in [`src/api.rs`](src/api.rs), then follow [`src/orchestrator.rs`](src/orchestrator.rs) and the affected layer. Treat `RuleEffect::sanitize`, the independent L1/L2 gates, `KernelSnapshot::migrate`, audit event compatibility, and persistence ordering as compatibility-sensitive surfaces.

## Documentation map

| Document | Focus |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Layer model, safety gates, and persistence design |
| [`docs/API_0.5.md`](docs/API_0.5.md) | Public API, schema, replay, and events |
| [`docs/INTEGRATING.md`](docs/INTEGRATING.md) | Adapters, configuration, and embedding |
| [`docs/CLOSED_LOOP.md`](docs/CLOSED_LOOP.md) | Safety switches and trajectory behavior |
| [`docs/DASHBOARD.md`](docs/DASHBOARD.md) | Operator dashboard workflows |
| [`docs/PORTABILITY.md`](docs/PORTABILITY.md) | `no_std`, WASM, and embedded portability |
| [`docs/L2_BENCHMARK.md`](docs/L2_BENCHMARK.md) | Reproducible L2 baseline comparison |
| [`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md) | Installation and runtime troubleshooting |
| [`CHANGELOG.md`](CHANGELOG.md) | Release history and unreleased changes |
| [`memory-bank/`](memory-bank/README.md) | Source-grounded project continuity context |
| [`ROADMAP.md`](ROADMAP.md) | Next development phases, priorities, and exit criteria |

## Roadmap and project boundaries

The 0.5 roadmap marks schema/replay, event hooks, strategy weights, dashboard workspaces, CLI parity, and status JSON as shipped. A full multi-tenant control plane, workspace multi-kernel orchestration service, and possible additional docs.rs feature-flag polish remain deferred.

The public core is released under the MIT License. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE) for licensing, trademark, and open-core details. Hosted services, managed policy packs, and enterprise control-plane features are not assumed to be part of the public MIT core.

## License

This project is released under the **MIT License**. The names “Omega”, “Omega Kernel”, and “Omega Cognitive Kernel” and related branding are trademarks of the copyright holder(s); source-code licensing does not grant trademark rights.

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
