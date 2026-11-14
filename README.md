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
| **[docs/API_0.5.md](docs/API_0.5.md)** | 0.5 surface (schema + replay + events) |
| **[docs/ROADMAP_0.5.md](docs/ROADMAP_0.5.md)** | 0.5 roadmap |
| **[docs/CRATES_IO.md](docs/CRATES_IO.md)** | Optional crates.io publish |

---

**Status:** research prototype (**v0.5.6**). Schema versioned; trajectory replay; event stream; strategy weight; multi-workspace dashboard; `status --json`; examples in crate package.

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.6 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.6 --locked --features gui --bin omega-dashboard
```

Or download binaries from [Releases](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/releases).

Library dependency (git):

```toml
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.6" }
```

crates.io publish is optional; see [docs/CRATES_IO.md](docs/CRATES_IO.md).

---

## Quick start

```bash
cargo run --release -- demo
cargo run --release -- run --steps 20
cargo run --release -- status
cargo run --release -- status --json
cargo run --release -- audit --summary
cargo run --release -- replay --tail 15
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
| **0.5.6** | examples in crate package (crates.io-ready) — **current** |

See [CHANGELOG.md](CHANGELOG.md).

---

## License & Brand

This project is released under the **MIT License** — see [LICENSE](LICENSE).

The names "Omega", "Omega Kernel", and "Omega Cognitive Kernel", together with related branding, are trademarks of the copyright holder(s). The MIT license grants rights to the *source code*; it does **not** grant trademark rights. See [NOTICE](NOTICE) for the full trademark and open-core posture statement.

Commercial plugins, hosted multi-tenant services, managed policy packs, and enterprise control-plane features may be offered under separate terms in the future.

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
