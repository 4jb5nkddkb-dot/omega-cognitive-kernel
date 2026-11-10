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

It is designed for systems that need to adapt *while running*, under tight safety and audit requirements.

| Doc | Topic |
|-----|--------|
| **[docs/PRODUCT.md](docs/PRODUCT.md)** | Positioning & monetization posture |
| **[docs/INTEGRATING.md](docs/INTEGRATING.md)** | Embed + adapters + config |
| **[docs/INBOX_VERTICAL.md](docs/INBOX_VERTICAL.md)** | Inbox vertical (events + audit) |
| **[docs/TOOL_GATE.md](docs/TOOL_GATE.md)** | **Tool-call gate** (allow/deny + optional real `http_get`) |
| **[docs/AUDIT_PANEL.md](docs/AUDIT_PANEL.md)** | Operator `audit --summary` |
| **[docs/INTERVENTION_POLICY.md](docs/INTERVENTION_POLICY.md)** | exploration vs strict intervention |
| **[docs/API_FREEZE_0.4.md](docs/API_FREEZE_0.4.md)** | Integrator API freeze (**v0.4.0+**) |

---

## What this is / is not

| Is | Is not |
|----|--------|
| Structured online patches (`RuleEffect`) | Hot-loading arbitrary source code |
| Auditable, checkpointable adaptation | Full industrial control / AGI |
| Pluggable observe/execute + tool gate | NLP content moderation product |
| L2 **heuristic** meta soft-nudge | License to bypass L0/L1 safety |

**Status:** research prototype (**v0.4.3**). Integrator API frozen for **0.4.x**.

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --locked
# pin:
# cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.3 --locked
```

From source: `cargo test && cargo build --release` → `target/release/omega-kernel`.

---

## Quick start

```bash
cargo run -- demo
cargo run -- run --steps 20
cargo run -- status
cargo run -- --config examples/omega.example.toml status
cargo run -- audit --summary
cargo run -- task --episodes 6 --steps 4
```

### Product demos

```bash
# Inbox vertical — events, policy, audit trail
cargo run --example inbox_vertical
KEEP_INBOX=1 cargo run --example inbox_vertical

# Tool-call gate — allow/deny (no network)
cargo run --example tool_gate

# Tool gate + real read-only HTTP GET after ALLOW
cargo run --example tool_gate_http
```

Docs: [INBOX_VERTICAL](docs/INBOX_VERTICAL.md) · [TOOL_GATE](docs/TOOL_GATE.md)

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

Adapters: `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, **`ToolGateExecutor`** (optional `enable_http_get`).  
Inject via `Kernel::with_adapters`.

---

## Safety posture

- L0 action whitelist + `max_risk`; **tool gate** for proposed tool names
- After ALLOW, optional real **`http_get`** only (`http`/`https`, truncated body); DENY never opens sockets
- L1 bounded `RuleEffect` + cooldown; checkpoints before patches
- L2 soft biases only
- Audit log + `audit --summary` for operators
- `[intervention]` policy: exploration (DX) vs strict (high-stakes)

---

## Status

| Stage | Meaning |
|-------|---------|
| **0.4.0** | Integrator API freeze |
| **0.4.1** | Inbox vertical demo |
| **0.4.2** | Tool gate + audit panel + intervention config |
| **0.4.3** | Real `http_get` after tool-gate ALLOW |
| **Next** | Thin service / dashboard when needed |

See [CHANGELOG.md](CHANGELOG.md).

---

## License

MIT — see [LICENSE](LICENSE).

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
