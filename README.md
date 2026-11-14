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
| **[docs/API_0.5.md](docs/API_0.5.md)** | 0.5 surface (schema + replay) |
| **[docs/ROADMAP_0.5.md](docs/ROADMAP_0.5.md)** | 0.5 roadmap |
| **[docs/API_FREEZE_0.4.md](docs/API_FREEZE_0.4.md)** | Historical 0.4 freeze notes |

---

## What this is / is not

| Is | Is not |
|----|--------|
| Structured online patches (`RuleEffect`) | Hot-loading arbitrary source code |
| Auditable, checkpointable adaptation | Full industrial control / AGI |
| Pluggable observe/execute + tool gate | NLP content moderation product |
| Optional desktop dashboard | Production public API gateway |

**Status:** research prototype (**v0.5.0**). Schema versioned; trajectory replay available.

---

## Install

```bash
# CLI
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.0 --locked

# Optional GUI dashboard
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.0 --locked --features gui --bin omega-dashboard
```

Or download binaries from the [Releases](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/releases) page.

---

## Quick start

```bash
cargo run --release -- demo
cargo run --release -- run --steps 20
cargo run --release -- status
cargo run --release -- audit --summary
cargo run --release -- replay --tail 15

# Desktop dashboard
cargo run --release --features gui --bin omega-dashboard
```

Default: **`self_modify_enabled = false`** (no L1 rule commits until you turn it on in config / dashboard).

### Product demos

```bash
cargo run --example inbox_vertical
cargo run --example tool_gate
cargo run --example tool_gate_http
cargo run --example audit_http_shim
```

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

Adapters: `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, `ToolGateExecutor`.

---

## Safety posture

- **`self_modify_enabled`** master switch (default off)
- Tool gate allow/deny; optional real `http_get` after ALLOW only
- Bounded `RuleEffect`, checkpoints, audit trail, optional `trajectory.jsonl`
- `[intervention]` exploration vs strict

---

## Version map

| Version | Meaning |
|---------|---------|
| **0.4.0–0.4.9** | Integrator freeze line + PER / dashboard / CI stability |
| **0.5.0** | Schema migrate + trajectory replay — **current** |

See [CHANGELOG.md](CHANGELOG.md).

---

## License

MIT — see [LICENSE](LICENSE).

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
