# Contributing to Omega Cognitive Kernel

Thank you for contributing to Omega Cognitive Kernel. The project is a research prototype for auditable, online strategy adaptation, so correctness, reversibility, persistence behavior, and clear documentation are as important as feature development.

Please read [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before participating. For suspected security vulnerabilities, follow [`SECURITY.md`](SECURITY.md) instead of opening a public issue.

## Getting started

### Prerequisites

- Rust `1.74+` and `rustup` for Rust development.
- Python `3.10+` for Python bindings.
- Git.
- Docker, optionally, for manylinux wheel testing.

### Set up the repository

```bash
git clone https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel.git
cd omega-cognitive-kernel
rustup update stable
rustup component add rustfmt clippy
```

For Python binding work, create an isolated environment and install the local build/test tools:

```bash
python -m venv .venv
. .venv/bin/activate                 # Windows: .venv\Scripts\activate
python -m pip install 'maturin>=1.4,<2' pytest
```

Build and run the default Rust checks:

```bash
cargo build --release
cargo test --locked
```

For Python binding work, build the extension from the repository root and run its tests:

```bash
maturin develop --manifest-path bindings/python/Cargo.toml --release
python -m pytest
```

## Development workflow

### Create a branch

Start from an up-to-date `main` branch:

```bash
git checkout main
git pull --ff-only origin main
git checkout -b feature/your-feature-name
```

Use a descriptive prefix where practical:

| Prefix | Use |
|---|---|
| `feature/` | New behavior or capability |
| `fix/` | Bug or regression fix |
| `docs/` | Documentation-only change |
| `refactor/` | Internal restructuring without intended behavior change |
| `test/` | Test additions or improvements |
| `ci/` | Workflow or build automation changes |

### Understand the compatibility-sensitive surfaces

Before changing runtime behavior, start at `Kernel::step` in [`src/api.rs`](src/api.rs), follow [`src/orchestrator.rs`](src/orchestrator.rs), and inspect the affected L0/L1/L2 or persistence module. Treat the following as compatibility-sensitive:

- `RuleEffect::sanitize` and its bounded action/effect fields.
- The independent `self_modify_enabled` and `l2.enabled` gates.
- `KernelSnapshot::migrate` and `CURRENT_SCHEMA_VERSION`.
- `AuditEvent` variants and JSONL compatibility.
- Audit, trajectory, checkpoint, and autosave ordering.
- Rust feature combinations and the declared Rust `1.74` MSRV.

If a change modifies public API, serialized state, audit events, safety defaults, or feature behavior, explain the compatibility and migration implications in the PR.

### Code and documentation style

For Rust changes, use standard Rust conventions, run `cargo fmt`, keep the MSRV in mind, and add regression tests for new behavior. For Python changes, follow PEP 8, use type hints where useful, and add or update pytest coverage. Public API changes must update the relevant API/integration documentation.

When project behavior or architecture changes, update the smallest relevant files under [`memory-bank/`](memory-bank/README.md). Keep Memory Bank claims grounded in source or executed checks; do not record credentials, private data, generated runtime state, or task transcripts.

## Verification commands

Run the narrowest relevant checks during iteration, then run the full applicable set before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

For feature or portability changes, also use the relevant checks:

```bash
cargo test --all-features
bash scripts/test-no-std.sh
bash scripts/test-wasm-runtime.sh
```

For Python binding changes:

```bash
maturin develop --manifest-path bindings/python/Cargo.toml --release
python -m pytest
```

For release-related changes, follow [`RELEASING.md`](RELEASING.md) and run the version/package checks documented there. If an external dependency, platform, registry, or workflow is unavailable, report that fact rather than treating the check as passed.

## Pull request checklist

Before opening a pull request, confirm the following:

### Scope and implementation

- [ ] The PR description explains the problem, the approach, and any user-visible behavior change.
- [ ] The change is limited to the stated scope and does not include unrelated generated files, local state, credentials, or backup artifacts.
- [ ] Public API, feature flags, safety gates, persistence schema, or audit-event changes are explicitly called out.
- [ ] Any schema or audit compatibility change includes migration, fallback, or consumer-impact notes.

### Tests and verification

- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --locked --all-targets -- -D warnings` passes for applicable Rust changes.
- [ ] `cargo test --locked` passes.
- [ ] Relevant `--all-features`, no-std, WASM, async, GUI, or Python checks have been run when affected.
- [ ] The PR description records exact commands that were run and any checks that could not be run.
- [ ] New regressions have a focused test, or the PR explains why a test is not practical.

### Documentation and project context

- [ ] README/API/integration/release documentation is updated for user-visible changes.
- [ ] `CHANGELOG.md` is updated for notable behavior, API, schema, safety, or release changes.
- [ ] Memory Bank files are updated when project goals, active work, architecture, technical constraints, or progress have changed.
- [ ] Examples, configuration snippets, and command names have been checked against the current source.
- [ ] New documentation has working relative links and does not claim unexecuted verification as successful.

### Safety and operations

- [ ] Policy changes remain bounded and do not introduce arbitrary code execution.
- [ ] L1 and L2 gate semantics remain explicit and tested if touched.
- [ ] Persistence failures remain observable and rollback/checkpoint behavior is covered if touched.
- [ ] Untrusted input, network access, tool execution, and sensitive audit/trajectory data are considered.
- [ ] Security-sensitive details are handled privately according to [`SECURITY.md`](SECURITY.md).

### Review readiness

- [ ] The branch is based on the current `main`.
- [ ] The working tree contains only intended changes.
- [ ] Commit messages follow the Conventional Commits format.
- [ ] The PR is ready for review, or clearly labels any draft/design questions.

## Commit messages

Use clear, descriptive messages following [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <subject>

<body>

<footer>
```

Common types are `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, and `ci`. Examples:

```text
docs(readme): clarify version mapping

Distinguish between the source package version and repository tags.
```

```text
feat(l2): add consequence-based defensive mode

Activate defensive mode after consecutive negative consequences and add
regression coverage for recovery after a strong positive consequence.
```

## Automated workflows

The repository includes CI, portability, release, weekly runtime-report, and formatting workflows under [`.github/workflows/`](.github/workflows/). A local Cline task-completion auto-commit hook is optional and documented in [`hooks/cline/README.md`](hooks/cline/README.md); do not rely on it as a substitute for reviewing or verifying changes.

The formatting workflow is a GitHub push workflow, not a local validation service. Commit and push reviewed implementation changes first, and use the resulting CI status as an additional signal rather than as a replacement for local checks.
