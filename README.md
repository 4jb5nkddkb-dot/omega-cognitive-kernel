# Omega Cognitive Kernel

**Self-improving cognitive runtime** with a three-layer loop and measurable consequence.

Product thesis: the system does not only *store* self-modifications — committed rules change the next cycle’s predictions and actions, and average prediction error can be shown to fall.

```
World (L0) → Cognitive upgrade (L1) → Metacognition (L2) → bias next L0
```

## Install / build

```bash
cargo build --release
cargo test
```

Binary name: `omega-kernel`.

## CLI (product surface)

```bash
# Narrative demo (isolated temp state)
cargo run -- demo

# Continue training against persistent state (.omega/state.json)
cargo run -- run --steps 20
cargo run -- status
cargo run -- run --goal "stabilize under uncertainty" --steps 5

# Custom state file
cargo run -- --state ./my-agent.json run --steps 10
cargo run -- --state ./my-agent.json status
cargo run -- --state ./my-agent.json reset --yes
```

### What `demo` proves

1. Kernel starts with relatively high uncertainty.
2. High error triggers L1 patches that carry a structured **`RuleEffect`** (not blind code eval).
3. Effect lowers error damping and reshapes action preference (`stabilize` / `observe` / …).
4. Report shows early vs final `avg_err` and whether error decreased.

## Library API

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut kernel = Kernel::open(None)?; // loads .omega/state.json if present
    let report = kernel.run("maintain coherent self-model", 10)?;
    println!(
        "err {:.3} → {:.3} (decreased={})",
        report.early_avg_err, report.final_avg_err, report.error_decreased
    );
    kernel.save()?;
    Ok(())
}
```

Key types: `Kernel`, `RunReport`, `KernelStatus`, `KernelSnapshot`.

## Architecture

| Layer | Role |
|-------|------|
| **L0** | Observe → predict (memory-aware) → decide (strategy + RuleEffect) → sandboxed execute |
| **L1** | Distill → reflect → sandbox → commit **RuleEffect** into memory |
| **L2** | Metrics → emergence → `omega=dayan` alignment → modulate biases/constraints |

### Safety

- L0 actions are whitelist + constraint checked.
- L1 patches: risk threshold + forbidden keywords; runtime applies **structured effects**, not arbitrary `eval`.
- L2 modulates biases only.

### Persistence

Default path: `.omega/state.json` (cases, strategies, CAS, active modules, patch history, biases).

## Project layout

```
src/
├── api.rs           # Kernel product API
├── persist.rs       # Snapshot load/save
├── main.rs          # CLI
├── orchestrator.rs
├── memory/  l0/  l1/  l2/  safety/
```

## Status

| Stage | Meaning |
|-------|---------|
| **S1** | Self-modification has measurable consequence |
| **S2** | Persist + CLI + library API + shareable demo narrative |
| Next | Hardening (audit log, rollback UX, one real task environment) |

---

*Omega Cognitive Kernel — from observation to accountable self-improvement.*
