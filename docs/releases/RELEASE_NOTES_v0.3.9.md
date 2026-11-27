# Omega Cognitive Kernel v0.3.9

**Training-free online strategy-patch runtime** — config file + more real-world observers.

Still a **research prototype**. L2 fixed-point language remains heuristic.

---

## Highlights

### TOML config (`omega.toml`)

- `KernelConfig` with load/save
- Knobs: state path, autosave, auto-checkpoint, L1 error threshold & patch cooldown, `max_risk`
- Library: `Kernel::apply_config`, `Kernel::from_config_path`
- CLI: `omega-kernel --config ./omega.toml status`

### New observers

- `DirSnapshotObserver` — offline directory monitoring
- `HttpObserver` — GET URL as structured observation

### Example

```bash
cargo run --example config_and_dir
```

See `examples/omega.example.toml`.

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.9 --locked
```

Binary also attached as `omega-kernel-linux-x86_64.tar.gz` when the release workflow completes.

---

## Quick check

```bash
omega-kernel demo
omega-kernel --config examples/omega.example.toml status
omega-kernel run --steps 12
cargo run --example config_and_dir
```

---

## Compatibility

- Default kernel / CLI paths unchanged without `--config`
- New APIs additive
- Supported surface: `Kernel`, config, adapters (`Observer`/`Executor` + File/Dir/Http/Logging), checkpoint/audit

See [CHANGELOG.md](./CHANGELOG.md).
