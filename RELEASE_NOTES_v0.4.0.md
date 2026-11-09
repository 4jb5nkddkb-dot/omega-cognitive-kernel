# Omega Cognitive Kernel v0.4.0

**API surface freeze** for the integrator-facing product line.

Still a **research prototype** for cognitive claims (L2 fixed-point language remains heuristic). The *API contract* for embedding is what 0.4 stabilizes.

---

## What “freeze” means

For **0.4.x** we intend **not** to rename or remove:

- `Kernel` construction / config / run / persist / checkpoint / audit methods listed in `docs/API_FREEZE_0.4.md`
- `KernelConfig` fields
- `Observer` / `Executor` and shipped adapters (File, Dir, HTTP, Logging, Sandboxed)
- CLI `--state`, `--config`, and existing subcommands

Still free to change: `kernel.inner`, non-exported internals, heuristics, examples.

---

## Highlights since 0.3.8

- Public adapters + `with_adapters`
- TOML config + CLI `--config`
- Dir / HTTP observers
- Drop-inbox closed-loop example
- Product / integrating / freeze docs under `docs/`

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.0 --locked
```

**crates.io:** not required for this release — **git tags remain the supported install path**.

---

## Quick check

```bash
omega-kernel demo
omega-kernel --config examples/omega.example.toml status
cargo run --example drop_inbox_loop
```

---

## Docs

- [docs/PRODUCT.md](./docs/PRODUCT.md)
- [docs/INTEGRATING.md](./docs/INTEGRATING.md)
- [docs/API_FREEZE_0.4.md](./docs/API_FREEZE_0.4.md)
- [CHANGELOG.md](./CHANGELOG.md)
