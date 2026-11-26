# Integrating Omega Cognitive Kernel

Practical guide for embedding the kernel in another process or service.

Target source version: **v0.6.0**. See [crates.io](https://crates.io/crates/omega-cognitive-kernel) for the latest published release.

**Recommended first run:** [examples/README.md](../examples/README.md) → `cargo run --example drop_inbox_loop` then open the dashboard.

---

## 1. Add the dependency

For the published package, use `cargo add omega-cognitive-kernel`. Its registry version may not match this source tree or expose its newest APIs. To integrate this checkout, use a path dependency:

```toml
# Cargo.toml
[dependencies]
omega-cognitive-kernel = { path = "/path/to/omega-cognitive-kernel" }
```

For a remote source dependency, pin a verified full commit with `{ git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", rev = "<full-commit-sha>" }`. Do not assume a source version already has a matching Git tag or registry release.

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
use omega_cognitive_kernel::Kernel;

let mut k = Kernel::from_config_path("omega.toml")?;
```

`from_config_path` applies the TOML settings and loads the snapshot if the
configured state file already exists. `Kernel::open(Some(path))` has the same
resume-if-present behavior for a direct state path. When using custom adapters,
construct the kernel with `cfg.resolved_state_path()`, apply the config, then
call `load()` when that state path exists; the `drop_inbox_loop` example shows
this pattern.

---

## 4. Event stream (KernelEvent)

`KernelEvent` is a type alias of `AuditEvent`. Two complementary ways to consume it:

### 4a. In-process hook (same process as the kernel)

```rust
use omega_cognitive_kernel::{AuditEvent, Kernel};

let mut k = Kernel::open(Some("./agent-state.json"))?;
k.set_event_hook(|ev: &AuditEvent| {
    match ev {
        AuditEvent::Step { action, avg_err, .. } => {
            println!("step action={action} err={avg_err:.3}");
        }
        AuditEvent::PatchAccepted { damping, .. } => {
            println!("patch accepted damping={damping:.2}");
        }
        _ => {}
    }
});
// ... run steps ...
k.clear_event_hook();
```

### 4b. File tail (another process / language / long-lived reader)

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

CLI equivalent: `omega-kernel audit-poll --offset N`.

Runnable demo covering both paths:

```bash
cargo run --example event_hook_poll
```

Offline trajectory summary (no world re-exec):

```rust
let sum = k.replay_trajectory(20)?;
println!("rows={} patches={}", sum.rows, sum.patches);
```

---

## 5. Status export

```rust
let st = k.status(); // KernelStatus (serde)
```

CLI:

```bash
omega-kernel status           # human text
omega-kernel status --json    # machine-readable
```

Localhost HTTP demo: `cargo run --example audit_http_shim` → `GET /status`.

---

## 6. Operator strategy weight nudge

```rust
// Returns false if the strategy name is unknown.
let ok = k.set_strategy_weight("explore", 1.25);
```

CLI: `omega-kernel strategy-weight explore 1.25`  
Dashboard Strategies tab also exposes `+` / `−` / reset controls. See [DASHBOARD.md](./DASHBOARD.md).

---

## 7. Real observe / act adapters

| Type | Role |
|------|------|
| `Observer` | Feed world state each step |
| `Executor` | Apply chosen actions |
| Built-ins | `FileObserver`, `DirSnapshotObserver`, `HttpObserver`, `LoggingExecutor`, `SandboxedExecutor`, `ToolGateExecutor` |

### 7a. Async adapters (requires `async` feature)

| Type | Role |
|------|------|
| `AsyncObserver` | Async version of `Observer` for non-blocking I/O |
| `AsyncExecutor` | Async version of `Executor` for non-blocking action execution |
| Built-ins | `AsyncFileObserver`, `AsyncDirSnapshotObserver`, `AsyncHttpObserver`, `AsyncLoggingExecutor` |
| Adapters | `AsyncObserverAdapter<O>`, `AsyncExecutorAdapter<E>` — wrap any sync impl |

Full list and quick-start path: [examples/README.md](../examples/README.md).

### 7b. Quick-start: async observers

```bash
# Async file/directory/HTTP observers demo
cargo run --example async_observer_demo --features async

# With HTTP support (requires reqwest feature)
cargo run --example async_observer_demo --features "async,reqwest"
```

### 7c. Quick-start: async executors

```bash
# Async executors & logging demo
cargo run --example async_executor_demo --features async
```

### 7d. Bridging sync → async

Any synchronous `Observer` or `Executor` can be used in an async context
by wrapping it with the provided adapters:

```rust
use omega_cognitive_kernel::{
    AsyncObserverAdapter, AsyncExecutorAdapter,
    FileObserver, SandboxedExecutor,
};

// Sync observer → async
let sync_observer = FileObserver::new("sensor.json");
let async_observer = AsyncObserverAdapter::new(sync_observer);

// Sync executor → async
let sync_executor = SandboxedExecutor::new(vec!["observe".into()]);
let async_executor = AsyncExecutorAdapter::new(sync_executor);
// Or simply: let async_executor: AsyncExecutorAdapter<_> = sync_executor.into();
```

The adapters run the blocking call on a tokio thread pool (`spawn_blocking`),
so they integrate seamlessly with async runtimes without blocking the event loop.

---

## 8. Safety contract

- Runtime policy surface is **`RuleEffect` only**
- L0 actions should stay on a **whitelist**
- Prefer **`self_modify_enabled = false`** until deliberate experiments
- Use **checkpoint** / **rollback**; inspect **audit** and **replay**

---

## 9. Persistence layout

- `state.json` — schema-versioned snapshot
- `audit.jsonl` — event stream (`KernelEvent`)
- `trajectory.jsonl` — optional step dump
- `checkpoints/` — rollback snapshots

---

## 10. Further reading

- [PRODUCT.md](./PRODUCT.md)
- [DASHBOARD.md](./DASHBOARD.md)
- [PORTABILITY.md](./PORTABILITY.md)
- [examples/README.md](../examples/README.md)
- [API_0.5.md](./API_0.5.md)
- [ROADMAP_0.5.md](./ROADMAP_0.5.md)
- [ARCHITECTURE.md](../ARCHITECTURE.md)
- [NOTICE](../NOTICE) — trademark / open-core boundary
- [CRATES_IO.md](./CRATES_IO.md)
