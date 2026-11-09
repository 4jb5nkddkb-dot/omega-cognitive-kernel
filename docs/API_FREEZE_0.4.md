# API freeze notes toward 0.4

**Goal:** make the *supported integrator surface* boring and stable so downstream crates can depend without chasing renames.

This is a **planning document**, not a promise of calendar dates. Current released line: **0.3.9**.

---

## Freeze candidates (intend to keep stable in 0.4.x)

### Crate root re-exports

- `Kernel`, `KernelStatus`, `RunReport`
- `KernelConfig`
- `OmegaError`, `Result`
- `Observer`, `Executor`
- Built-in adapters that are already public:
  - `MultiModalObserver`, `FileObserver`, `DirSnapshotObserver`, `HttpObserver`
  - `SandboxedExecutor`, `LoggingExecutor`
- `run_bandit_task`, `BanditReport` (metrics fields may grow; existing fields stay)

### `Kernel` methods

- Construction: `new`, `with_state_path`, `open`, `with_adapters`, `with_adapters_and_path`, `from_config_path`
- Config: `apply_config`
- Loop: `step`, `run`, `status`
- Persist: `save`, `load`, `snapshot`, `apply_snapshot`
- Safety ops: `checkpoint`, `list_checkpoints`, `rollback`, `audit_tail`, `audit_task`
- Default kernel: `reset`

### `KernelConfig` fields

- `state_path`, `autosave`, `auto_checkpoint_on_patch`
- `error_threshold`, `min_steps_between_patches`, `max_risk`
- `default_goal`

New fields may be added with `serde` defaults; removals require a major bump.

### CLI (user-facing)

- Global: `--state`, `--config`
- Commands: `run`, `status`, `demo`, `task`, `audit`, `checkpoints`, `checkpoint`, `rollback`, `reset`

Flag renames should be avoided; new flags OK.

---

## Explicitly **unstable** (may change without 0.4 major)

- Anything under `kernel.inner` / `OmegaOrchestrator` field layout
- L1/L2 internal types not re-exported at crate root
- Exact patch generator heuristics and numeric constants (behavior may improve)
- Persistence file schema beyond “load what this version wrote” (may add versioned migrations)
- Example code in `examples/` (illustrative)

---

## Before calling it 0.4.0

Checklist:

1. [ ] No outstanding renames on freeze candidates
2. [ ] README + INTEGRATING.md match the freeze list
3. [ ] At least one vertical example remains green (`drop_inbox_loop`)
4. [ ] `cargo test` + release workflow green on tag
5. [ ] CHANGELOG section “0.4.0 — API surface freeze”
6. [ ] Decide crates.io: publish 0.4.0 or stay git-tag only

---

## Semver policy (proposed)

| Change | Version |
|--------|---------|
| Break freeze-list signature / remove config field | `0.5` or `1.0` |
| Add optional config field / new adapter / new status field | `0.4.x` |
| Bugfix, heuristic tweak, docs | `0.4.x` or patch |

Until 1.0, minors may still carry careful breaks **outside** the freeze list; document them in CHANGELOG.

---

## Out of scope for 0.4

- Hosted dashboard / multi-tenant control plane
- Guarantees about L2 “fixed point” semantics beyond heuristic status fields
- Replacing the RuleEffect model with free-form code patches
