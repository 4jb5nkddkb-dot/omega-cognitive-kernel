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
| **[docs/INBOX_VERTICAL.md](docs/INBOX_VERTICAL.md)** | Inbox vertical (events + audit) |
| **[docs/TOOL_GATE.md](docs/TOOL_GATE.md)** | Tool-call gate (+ optional real `http_get`) |
| **[docs/AUDIT_PANEL.md](docs/AUDIT_PANEL.md)** | Operator `audit --summary` |
| **[docs/AUDIT_HTTP_SHIM.md](docs/AUDIT_HTTP_SHIM.md)** | **Localhost read-only audit HTTP** |
| **[docs/INTERVENTION_POLICY.md](docs/INTERVENTION_POLICY.md)** | exploration vs strict intervention |
| **[docs/API_FREEZE_0.4.md](docs/API_FREEZE_0.4.md)** | Integrator API freeze (**v0.4.0+**) |

---

## What this is / is not

| Is | Is not |
|----|--------|
| Structured online patches (`RuleEffect`) | Hot-loading arbitrary source code |
| Auditable, checkpointable adaptation | Full industrial control / AGI |
| Pluggable observe/execute + tool gate | NLP content moderation product |
| Localhost audit HTTP **demo** | Production public API gateway |

**Status:** research prototype (**v0.4.4**). Integrator API frozen for **0.4.x**.

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --locked
# pin:
# cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.4 --locked
```

---

## Quick start

```bash
cargo run -- demo
cargo run -- run --steps 20
cargo run -- audit --summary
```

### Product demos

```bash
# Inbox vertical
cargo run --example inbox_vertical

# Tool gate (deny high-risk names)
cargo run --example tool_gate

# Tool gate + real GET after ALLOW
cargo run --example tool_gate_http

# Localhost audit HTTP shim (read-only)
cargo run --example audit_http_shim
# curl -s http://127.0.0.1:8787/audit/summary
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
    kernel.save()?;
    Ok(())
}
```

Adapters: `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, `ToolGateExecutor`.

---

## Safety posture

- Tool gate allow/deny; optional real `http_get` after ALLOW only
- Bounded `RuleEffect`, checkpoints, audit trail
- `[intervention]` exploration vs strict
- Audit HTTP shim is **localhost demo only** (no auth/TLS)

---

## Status

| Stage | Meaning |
|-------|---------|
| **0.4.0** | Integrator API freeze |
| **0.4.1** | Inbox vertical |
| **0.4.2** | Tool gate + audit panel + intervention |
| **0.4.3** | Real `http_get` after ALLOW |
| **0.4.4** | Localhost audit HTTP shim |
| **Next** | Real caller-driven service/auth only when needed |

See [CHANGELOG.md](CHANGELOG.md).

---

## License

MIT — see [LICENSE](LICENSE).

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
