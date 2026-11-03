# Omega Cognitive Kernel

**Self-improving cognitive runtime** with a three-layer loop and measurable consequence.

Product thesis: committed rules change the next cycle’s predictions and actions; average prediction error can be shown to fall.

```
World (L0) → Cognitive upgrade (L1) → Metacognition (L2) → bias next L0
```

## Install / build

```bash
cargo build --release
cargo test
```

Binary: `omega-kernel` (v0.3.0).

## CLI

```bash
cargo run -- demo
cargo run -- run --steps 20
cargo run -- status
cargo run -- audit --tail 30
cargo run -- checkpoints
cargo run -- checkpoint --label before-experiment
cargo run -- rollback <UUID>
cargo run -- task --episodes 6 --steps 4
cargo run -- --state ./my-agent.json run --steps 10
cargo run -- reset --yes
```

## S3 Hardening

| Feature | CLI | Notes |
|---------|-----|-------|
| Audit log | `audit --tail N` | Append-only JSONL beside state |
| Checkpoints | `checkpoints` | Auto **pre-patch** snapshots |
| Manual checkpoint | `checkpoint --label …` | |
| Rollback | `rollback <UUID>` | Restores snapshot + audit event |
| Bandit task | `task` | Nonstationary 2-arm external score |

Layout:

```
.omega/
  state.json
  audit.jsonl
  checkpoints/
    <uuid>.meta.json
    <uuid>.state.json
```

## Library API

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut kernel = Kernel::open(None)?;
    let report = kernel.run("maintain coherent self-model", 10)?;
    println!("{:?}", report);
    let ckpt = kernel.checkpoint("stable")?;
    kernel.rollback(ckpt.id)?;
    Ok(())
}
```

## Architecture

| Layer | Role |
|-------|------|
| **L0** | Observe → predict → decide → sandboxed execute |
| **L1** | Distill → reflect → sandbox → commit **RuleEffect** |
| **L2** | Metrics → emergence → omega=dayan → modulate biases |

## Status

| Stage | Meaning |
|-------|---------|
| **S1** | Self-modification has measurable consequence |
| **S2** | Persist + CLI + library API + demo narrative |
| **S3** | Audit, checkpoints/rollback, bandit task |
| Next | Stronger tasks, packaging, optional sensors |

---

*Omega Cognitive Kernel — from observation to accountable self-improvement.*
