# Omega Cognitive Kernel — Python bindings

Python bindings for the training-free, auditable `RuleEffect` adaptation runtime. The package version is read dynamically from `bindings/python/Cargo.toml`, keeping it aligned with the Rust source.

**Status:** research prototype, source version **0.6.1**. The default observer and executor are simulations; L1 self-modification is disabled by default and bounded L2 modulation is enabled.

## Install

```bash
python -m pip install omega-cognitive-kernel
```

The published distribution name is `omega-cognitive-kernel`; the import name is `omega_cognitive_kernel_python`.

## Quick start

```python
import omega_cognitive_kernel_python as ok

kernel = ok.Kernel.with_state_path("./agent-state.json")
report = kernel.run("maintain coherent self-model", 10)
print(
    f"error {report.early_avg_err:.3f} -> {report.final_avg_err:.3f}; "
    f"decreased={report.error_decreased}"
)

checkpoint = kernel.create_checkpoint("before-review")
kernel.run("try another policy", 3)
kernel.rollback(checkpoint.id)
kernel.save()
```

`Kernel.open(path)` loads an existing state file or creates a new in-memory kernel at that path. Always use a dedicated state directory in applications and tests.

## Exposed API

- `Kernel`: `new`, `with_state_path`, `open`, `from_config_path`, `step`, `run`, `status`, `save`, `snapshot_json`, `apply_snapshot_json`.
- Safety and policy controls: `create_checkpoint`, `list_checkpoints`, `rollback`, `set_strategy_weight`, `set_l2_enabled`, `set_audit_max_bytes`.
- Runtime settings: `autosave`, `auto_checkpoint_on_patch`, and `dump_trajectory` getters/setters.
- Observability: `replay_trajectory`, `AuditLog.beside_state`, `read_all`, `tail`, `poll_after`, `summarize`.
- `CheckpointStore`: read persisted checkpoint metadata and snapshots.

The Python wrapper currently exposes the built-in simulated observer/executor, not custom Python observer or executor callbacks.

## Build and test from source

Run from the repository root:

```bash
python -m venv .venv
. .venv/bin/activate
python -m pip install 'maturin>=1.4,<2' pytest
maturin develop --manifest-path bindings/python/Cargo.toml --release
python -m pytest
```

To produce a wheel without installing it:

```bash
maturin build --manifest-path bindings/python/Cargo.toml --release --out /tmp/omega-wheelhouse
python -m pip install --force-reinstall /tmp/omega-wheelhouse/omega_cognitive_kernel-*.whl
python -m pytest
```

Do not use the wheel checked into older development branches as a test dependency; it is version- and interpreter-specific.

## Safety and limitations

- `self_modify_enabled` defaults to `false`.
- Audit JSONL appends default to a 64 MiB file ceiling. Use `kernel.set_audit_max_bytes(bytes)` before running steps to choose a different ceiling; exceeding it fails closed without truncating earlier records.
- Policy updates are bounded structured `RuleEffect` values, not arbitrary code execution.
- Use checkpoints, rollback, audit logs, and replay to inspect behavior.
- CPython 3.10–3.15 are officially supported and covered by CI and the release wheel workflow. The package minimum remains Python 3.10.
- For custom real-world adapters and async I/O, use the Rust API described in the [integration guide](../../docs/INTEGRATING.md).

## License

MIT for source code; see the repository `LICENSE` and `NOTICE` for license and trademark terms.
