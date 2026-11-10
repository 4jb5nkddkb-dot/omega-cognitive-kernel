# Changelog

## [0.4.2] — 2026-11-10

### Tool-call gate (safety MVP)

- `ToolGateExecutor` — allow-list + deny-list, optional audit, `fail_closed`
- Audit events: `ToolAllowed`, `ToolDenied`
- `AuditSummary.tools_allowed` / `tools_denied`
- Example: `examples/tool_gate.rs`
- Doc: `docs/TOOL_GATE.md`

### Audit panel & intervention policy

- CLI: `omega-kernel audit --summary` (counts + action histogram + intervention block)
- `AuditSummary` / `AuditLog::summarize` / `read_all`
- `[intervention]` in `omega.toml`: mode, max_level, friction_threshold
- Types: `InterventionPolicy`, `InterventionMode`, `InterventionLevel`
- Docs: `docs/AUDIT_PANEL.md`, `docs/INTERVENTION_POLICY.md`

**No breaks** to the 0.4.0 freeze list; additive APIs only.

## [0.4.1] — 2026-11-09

### Demo & docs

- `examples/inbox_vertical.rs` — event-queue Observer + policy-gate Executor, audit/actions evidence
- `docs/INBOX_VERTICAL.md` — how to run and honest scope
- README links inbox vertical as the recommended product demo

**No changes** to the 0.4.0 integrator freeze surface.

## [0.4.0] — 2026-11-09

### API surface freeze

- Declare the **integrator surface** stable for the 0.4.x line (see `docs/API_FREEZE_0.4.md`)
- Freeze candidates: `Kernel` (+ construction/config/loop/persist/safety methods), `KernelConfig`, `Observer`/`Executor` and built-in adapters, `run_bandit_task` / `BanditReport`, CLI `--state` / `--config` and existing subcommands
- Explicitly **unstable**: `kernel.inner` layout, non-re-exported L1/L2 internals, heuristic constants, example code
- Docs pack: `docs/PRODUCT.md`, `docs/INTEGRATING.md`, `docs/API_FREEZE_0.4.md`
- Product demo: `examples/drop_inbox_loop.rs`
- Packaging: `docs/**` included in crate `include` list
- **crates.io:** still **git-tag preferred**; no crates.io publish required for 0.4.0

No intentional breaking renames on the freeze list versus 0.3.9.

## [0.3.9] — 2026-11-09

### Config

- `KernelConfig` TOML load/save (`omega.toml`)
- Fields: `state_path`, `autosave`, `auto_checkpoint_on_patch`, `error_threshold`, `min_steps_between_patches`, `max_risk`, `default_goal`
- `Kernel::apply_config` / `Kernel::from_config_path`
- CLI global `--config path/to/omega.toml`
- Example: `examples/omega.example.toml`, `examples/config_and_dir.rs`

### Adapters

- `DirSnapshotObserver` — directory file listing (offline-friendly)
- `HttpObserver` — GET URL as structured observation (`ureq`)

### Deps

- `toml`, `ureq`

## [0.3.8] — 2026-11-09

### Adapter surface (breaking-compatible)

- `Observer` / `Executor` (and related L0 traits) re-exported from crate root
- `Kernel` is generic over `Observer` + `Executor` with **defaults unchanged**
  (`MultiModalObserver`, `SandboxedExecutor`)
- New: `Kernel::with_adapters` / `with_adapters_and_path` for external injection
- New adapters: `FileObserver`, `LoggingExecutor`
- Example: `examples/custom_adapters.rs`
- Product-oriented README lead-in

## [0.3.7] — 2026-11-05

### L2 biases → DecisionMaker

- Meta biases mapped onto action scores
- Bandit strategy-decay metrics

## [0.3.6] — [0.1.0]

- See prior tags for L0–L2 hardening, packaging, CLI
