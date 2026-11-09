# Changelog

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

- Meta biases mapped onto action scores:
  - `explore_preference` → boost explore / mild penalize exploit+stabilize
  - `self_ref` + `coherence` → boost stabilize (+ mild observe)
  - `observation_depth` → boost observe
- Action params expose `l2_explore_pref`
- Unit tests: explore_preference selects explore; self_ref+coherence selects stabilize

### Bandit strategy-decay metrics

- `StrategyMemory::stats` reports idle/active counts and weight means
- `BanditReport` adds: `strategy_count`, `idle_strategy_count`, `mean_strategy_weight`, `mean_idle_steps`, `max_idle_steps`, `mean_idle_weight`, `mean_active_weight`
- CLI `task` prints a strategy-decay block

## [0.3.6] — 2026-11-05

- L0 owns Learner path; single decay ownership; fixed-point exit e2e test

## [0.3.5] — 2026-11-05

- Strategy idle decay; Learner ownership; integration tests (cooldown, max_risk, L2 nudge)

## [0.3.4] — 2026-11-05

- L2 soft RuleEffect modulation; constraints in DecisionMaker; L2 status surface

## [0.3.3] — 2026-11-05

- L1 Abstractor/cooldown/store bound; L2 lifecycle

## [0.3.2] — 2026-11-05

- RuleEffect explore_bonus / risk_tolerance

## [0.3.1] — 2026-11-03

- Packaging, MIT, release workflow

## [0.3.0] — 2026-11-03

- Audit, checkpoints, bandit task

## [0.2.0] / [0.1.0]

- Persistence + CLI; L0/L1/L2 skeleton
