# Omega Cognitive Kernel

**Training-free online strategy adaptation** — patch a structured `RuleEffect`, measure the consequence, roll back if needed.

> When error rises, the kernel emits a bounded policy patch, persists it as cognitive state, and changes the next cycle’s decisions.  
> **No weight updates. No code execution. Full audit trail.**

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

## Why this exists

Most adaptive systems either:
- require expensive retraining / fine-tuning, or
- allow arbitrary code changes that are hard to audit and reverse.

Omega takes a third path: **structured, reversible policy patches**.

| Doc | Topic |
|-----|--------|
| **[docs/PRODUCT.md](docs/PRODUCT.md)** | Positioning & monetization posture |
| **[docs/INTEGRATING.md](docs/INTEGRATING.md)** | Embed + adapters + config |
| **[docs/CLOSED_LOOP.md](docs/CLOSED_LOOP.md)** | Safety switches + trajectory |
| **[docs/DASHBOARD.md](docs/DASHBOARD.md)** | Desktop operator UI |
| **[docs/PORTABILITY.md](docs/PORTABILITY.md)** | no_std / WASM / embedded portability |
| **[docs/API_0.5.md](docs/API_0.5.md)** | 0.5 surface (schema + replay + events) |
| **[docs/ROADMAP_0.5.md](docs/ROADMAP_0.5.md)** | 0.5 roadmap |
| **[docs/L2_BENCHMARK.md](docs/L2_BENCHMARK.md)** | Reproducible L2 baseline comparison |
| **[docs/CRATES_IO.md](docs/CRATES_IO.md)** | crates.io notes |
| **[docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md)** | Common issues and solutions |

---

**Status:** research prototype. The current source and Python binding version is **0.5.25**; the newest repository tag is **v0.5.30**. Check [crates.io](https://crates.io/crates/omega-cognitive-kernel) for the latest published library. Schema versioning, trajectory replay, event streaming, strategy weights, async adapters, a multi-workspace dashboard, and `status --json` are implemented.

---

## Install

**Library (crates.io):**

```bash
cargo add omega-cognitive-kernel
```

```toml
omega-cognitive-kernel = "0.5.25"
```

**CLI / dashboard from git tag:**

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.25 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.25 --locked --features gui --bin omega-dashboard
```

Or download binaries from [Releases](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/releases).

---

## Quick start

```bash
# Isolated demo: state, audit, and trajectory use a fresh temporary directory.
cargo run --release --bin omega-kernel -- demo

# Runtime commands share .omega/state.json unless --state selects another path.
cargo run --release --bin omega-kernel -- run --steps 20
cargo run --release --bin omega-kernel -- status
cargo run --release --bin omega-kernel -- status --json
cargo run --release --bin omega-kernel -- audit --summary
cargo run --release --bin omega-kernel -- replay --tail 15
cargo run --release --bin omega-kernel -- benchmark-l2 --runs 20 --json
cargo run --release --features gui --bin omega-dashboard
```

Default: **`self_modify_enabled = false`**.

---

## Library API

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut kernel = Kernel::open(None)?;
    let report = kernel.run("maintain coherent self-model", 10)?;
    println!(
        "err {:.3} → {:.3} (decreased={})",
        report.early_avg_err, report.final_avg_err, report.error_decreased
    );
    let replay = kernel.replay_trajectory(10)?;
    println!("trajectory rows={} patches={}", replay.rows, replay.patches);
    kernel.save()?;
    Ok(())
}
```

---

## Version map

| Version | Meaning |
|---------|---------|
| **0.4.x** | Integrator freeze + PER + dashboard intro |
| **0.5.0** | Schema migrate + trajectory replay |
| **0.5.1** | Dashboard Replay tab + required GUI release artifacts |
| **0.5.2–0.5.4** | Event hooks / poll_after, strategy weight, multi-workspace, CLI parity |
| **0.5.5** | `status --json` + HTTP `/status` example |
| **0.5.6** | Examples in crate package; first crates.io release |
| **0.5.7+** | Async adapters and CI hardening |
| **0.5.25** | Current source version (Cargo.toml and Python bindings) |
| **v0.5.30** | Newest repository tag (may include unreleased changes) |

## Python bindings

```bash
python -m pip install omega-cognitive-kernel
```

```python
import omega_cognitive_kernel_python as ok

kernel = ok.Kernel.with_state_path("./agent-state.json")
report = kernel.run("maintain coherent self-model", 10)
print(report)
kernel.save()
```

See [the Python binding guide](bindings/python/README.md) for local development and testing.
Package: [PyPI — omega-cognitive-kernel](https://pypi.org/project/omega-cognitive-kernel/).

See [CHANGELOG.md](CHANGELOG.md).

---

## License & Brand

This project is released under the **MIT License** — see [LICENSE](LICENSE).

The names "Omega", "Omega Kernel", and "Omega Cognitive Kernel", together with related branding, are trademarks of the copyright holder(s). The MIT license grants rights to the *source code*; it does **not** grant trademark rights. See [NOTICE](NOTICE) for the full trademark and open-core posture statement.

Commercial plugins, hosted multi-tenant services, managed policy packs, and enterprise control-plane features may be offered under separate terms in the future.

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
