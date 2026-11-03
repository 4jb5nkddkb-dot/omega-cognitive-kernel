# Omega Cognitive Kernel

**Online strategy-patch runtime** — lightweight, training-free adaptation with an auditable control loop.

> One line: when error rises, emit a structured **`RuleEffect` patch**, persist it as cognitive state, change the next cycle’s predictions and actions — **no weight update, no retrain**.

Classical control / RL supply the skeleton (observe → predict → decide → act, explore/exploit, stabilize under error). The engineering bet is different from fine-tuning or PPO: **patch the policy surface, remember the effect, measure the consequence, roll back if needed**.

```
World (L0) → Reflect & patch (L1) → Meta-modulate (L2) → bias next L0
```

### What this is / is not

| Is | Is not |
|----|--------|
| Structured online patches (`RuleEffect`: damping, action preference) | Hot-loading or executing arbitrary new source code |
| Durable cognitive state in memory + `state.json` | Full Bayesian filter / industrial PID / AGI |
| Auditable, checkpointable, reversible adaptation | “Emergence” as a proven fixed point |
| Minimal runtime (JSON state + small Kernel API) | A substitute for large-scale model training |

---

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
cargo run -- checkpoint --label before-x
cargo run -- rollback <UUID>
cargo run -- task --episodes 6 --steps 4
cargo run -- --state ./my-agent.json run --steps 10
cargo run -- reset --yes
```

### What `demo` proves

1. Kernel starts with relatively high uncertainty.
2. High error triggers L1 patches that carry a **`RuleEffect`** (not code `eval`).
3. Effect changes error damping and action preference.
4. Report shows early vs final `avg_err` and whether error decreased.

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
    let ckpt = kernel.checkpoint("stable")?;
    kernel.save()?;
    Ok(())
}
```

## Architecture

| Layer | Role |
|-------|------|
| **L0** | Observe → predict → decide (strategy + `RuleEffect`) → sandboxed execute |
| **L1** | Distill → reflect → validate/sanitize → commit **`RuleEffect`** |
| **L2** | Metrics → optional meta-mode → modulate biases/constraints (heuristic) |

### Safety posture

- L0: action whitelist + constraints
- L1: risk + keywords + **`RuleEffect` bounds**; runtime applies structured effects only
- Checkpoints (FIFO max 32) before accepted patches; rollback restores snapshot

### Persistence & audit

```
.omega/
  state.json
  audit.jsonl
  checkpoints/
```

## Design stance

**Mature ideas, unusual packaging.** The distinctive choice is a **training-free patch loop**: typed effect, durable memory, audit + checkpoint.

## Status

| Stage | Meaning |
|-------|---------|
| **S1** | Self-modification has measurable consequence |
| **S2** | Persist + CLI + library API + demo narrative |
| **S3** | Audit, checkpoints/rollback, bandit task shell |
| **Hardening** | Reward→strategy feedback, RuleEffect bounds, checkpoint FIFO, seeded bandit |
| Next | Stronger real tasks, packaging |

---

*Omega Cognitive Kernel — patch, remember, measure, roll back.*
