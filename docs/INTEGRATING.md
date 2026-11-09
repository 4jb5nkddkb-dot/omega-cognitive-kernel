# Integrating Omega Cognitive Kernel

Practical guide for embedding the kernel in another process or service.

Target version: **v0.3.9+**

---

## 1. Add the dependency

```toml
# Cargo.toml
[dependencies]
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.3.9" }
```

Or path / main branch while iterating.

---

## 2. Minimal embed (default simulated world)

```rust
use omega_cognitive_kernel::Kernel;

fn main() -> omega_cognitive_kernel::Result<()> {
    let mut k = Kernel::open(Some("./agent-state.json"))?;
    let report = k.run("keep prediction error under control", 10)?;
    println!(
        "err {:.3} → {:.3} decreased={}",
        report.early_avg_err, report.final_avg_err, report.error_decreased
    );
    k.save()?;
    Ok(())
}
```

---

## 3. Config file

Copy `examples/omega.example.toml` and tune:

```toml
state_path = "./agent-state.json"
autosave = true
auto_checkpoint_on_patch = true
error_threshold = 0.30
min_steps_between_patches = 3
max_risk = 0.65
default_goal = "stabilize under load"
```

Library:

```rust
use omega_cognitive_kernel::{Kernel, KernelConfig};

let cfg = KernelConfig::load("omega.toml")?;
let mut k = Kernel::with_state_path(cfg.resolved_state_path());
k.apply_config(&cfg);
```

CLI:

```bash
omega-kernel --config ./omega.toml status
omega-kernel --config ./omega.toml run --steps 12
```

`--state` overrides `state_path` from config.

---

## 4. Real observe / act adapters

Implement or reuse:

| Type | Role |
|------|------|
| `Observer` | Feed world state each step |
| `Executor` | Apply chosen actions (whitelist still enforced by your impl) |
| Built-ins | `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, `SandboxedExecutor` |

```rust
use omega_cognitive_kernel::{
    DirSnapshotObserver, Kernel, LoggingExecutor,
};

let observer = DirSnapshotObserver::new("./inbox");
let executor = LoggingExecutor::new([
    "noop".into(), "observe".into(), "explore".into(),
    "exploit".into(), "stabilize".into(),
])
.with_log_file("./actions.log");

let mut k = Kernel::with_adapters(observer, executor);
k.apply_config(&KernelConfig::load("omega.toml")?);
let _ = k.run("stabilize under inbox load", 8)?;
```

End-to-end story: `cargo run --example drop_inbox_loop`.

---

## 5. Safety contract (do not bypass)

- Runtime policy surface is **`RuleEffect` only** — not arbitrary code execution
- L0 actions should stay on a **whitelist**
- L1 patches are **bounded + cooldown**
- L2 only **soft-biases / soft constraints**
- Use **checkpoint** before risky experiments; **rollback** on regression
- Prefer public `Kernel` APIs over `kernel.inner.*`

---

## 6. Persistence layout

Beside your state file (or under `.omega/` depending on path helpers):

- `state.json` — cognitive snapshot
- `audit.jsonl` — per-step / patch / checkpoint events
- `checkpoints/` — rollback snapshots

---

## 7. Suggested integration checklist

1. [ ] Pin a **tag** (`v0.3.9` or later)
2. [ ] Ship an `omega.toml` with explicit `max_risk` and cooldown
3. [ ] Start with `LoggingExecutor` before real side effects
4. [ ] Wire one real `Observer` (file, dir, or HTTP)
5. [ ] Log `RunReport` / `KernelStatus` to your metrics system
6. [ ] Document how operators rollback a bad patch

---

## 8. Further reading

- [PRODUCT.md](./PRODUCT.md) — positioning and monetization posture
- [ARCHITECTURE.md](../ARCHITECTURE.md) — L0/L1/L2 data flow
- [API_FREEZE_0.4.md](./API_FREEZE_0.4.md) — what we intend to stabilize
