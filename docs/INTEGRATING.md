# Integrating Omega Cognitive Kernel

Practical guide for embedding the kernel in another process or service.

Target version: **v0.5.1+**

---

## 1. Add the dependency

```toml
# Cargo.toml
[dependencies]
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.1" }
```

Or path / main branch while iterating.

---

## 2. Minimal embed

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

```toml
state_path = "./agent-state.json"
autosave = true
auto_checkpoint_on_patch = true
self_modify_enabled = false
dump_trajectory = true
error_threshold = 0.30
min_steps_between_patches = 3
max_risk = 0.65
default_goal = "stabilize under load"
```

```rust
use omega_cognitive_kernel::{Kernel, KernelConfig};

let cfg = KernelConfig::load("omega.toml")?;
let mut k = Kernel::with_state_path(cfg.resolved_state_path());
k.apply_config(&cfg);
```

---

## 4. Event stream (KernelEvent)

`KernelEvent` is an alias of `AuditEvent`. Tail new records without re-reading the whole file:

```rust
use omega_cognitive_kernel::AuditLog;

let log = AuditLog::beside_state(std::path::Path::new("./agent-state.json"));
let mut cursor = log.byte_len()?;
loop {
    let (rows, next) = log.poll_after(cursor)?;
    for r in rows {
        println!("{} {:?}", r.ts, r.event);
    }
    cursor = next;
    std::thread::sleep(std::time::Duration::from_millis(200));
}
```

Offline trajectory summary (no world re-exec):

```rust
let sum = k.replay_trajectory(20)?;
println!("rows={} patches={}", sum.rows, sum.patches);
```

---

## 5. Real observe / act adapters

| Type | Role |
|------|------|
| `Observer` | Feed world state each step |
| `Executor` | Apply chosen actions |
| Built-ins | `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, `SandboxedExecutor`, `ToolGateExecutor` |

---

## 6. Safety contract

- Runtime policy surface is **`RuleEffect` only**
- L0 actions should stay on a **whitelist**
- Prefer **`self_modify_enabled = false`** until deliberate experiments
- Use **checkpoint** / **rollback**; inspect **audit** and **replay**

---

## 7. Persistence layout

- `state.json` — schema-versioned snapshot
- `audit.jsonl` — event stream (`KernelEvent`)
- `trajectory.jsonl` — optional step dump
- `checkpoints/` — rollback snapshots

---

## 8. Further reading

- [PRODUCT.md](./PRODUCT.md)
- [API_0.5.md](./API_0.5.md)
- [ROADMAP_0.5.md](./ROADMAP_0.5.md)
- [ARCHITECTURE.md](../ARCHITECTURE.md)
