# Omega Cognitive Kernel v0.5.7 Release Notes

**Release Date**: 2026-11-17  
**Crates.io**: `omega-cognitive-kernel = "0.5.7"`  
**Git Tag**: `v0.5.7`

---

## Highlights

### 🚀 Async I/O Support (feature = "async")

This release introduces first-class async I/O support for observers and executors, enabling non-blocking integration with tokio-based applications.

#### New Async Traits
- **`AsyncObserver`** — Async version of `Observer` for non-blocking world observation
- **`AsyncExecutor`** — Async version of `Executor` for non-blocking action execution

#### Adapter Bridging (Sync → Async)
- **`AsyncObserverAdapter<O>`** — Wrap any synchronous `Observer` to use as `AsyncObserver` (runs on tokio thread pool via `spawn_blocking`)
- **`AsyncExecutorAdapter<E>`** — Wrap any synchronous `Executor` to use as `AsyncExecutor`

#### Concrete Async Adapters
| Adapter | Description |
|---------|-------------|
| `AsyncFileObserver` | Async file reading using tokio fs |
| `AsyncDirSnapshotObserver` | Async directory snapshot using tokio fs |
| `AsyncHttpObserver` | Async HTTP requests (reqwest with ureq fallback) |
| `AsyncLoggingExecutor` | Async action logging with optional file output |

#### Quick Start
```bash
# Async observers demo
cargo run --example async_observer_demo --features async

# Async executors demo  
cargo run --example async_executor_demo --features async

# With HTTP support (requires reqwest)
cargo run --example async_observer_demo --features "async,reqwest"
```

### 📚 Documentation & Examples
- Added `async_observer_demo.rs` and `async_executor_demo.rs` examples
- Updated `docs/INTEGRATING.md` with async adapter usage guide
- Added `docsrs` feature for proper docs.rs feature-gated documentation rendering

### 🔧 CI / Infrastructure
- **L2 Baseline Regression job**: Nonstationary bandit benchmark (5 seeds) with JSON artifact upload for trend tracking
- Fixed `rfd` backend for Linux CI: `xdg-portal` + `tokio` (pure Rust, no GTK3 system dependencies)

### 🧠 L1 Improvements
- `PatchGenerator` now uses `high_error_streak` more aggressively (extra damping on long streaks)
- Includes streak + recent mean error in the audit `code` comment and `expected_effect` string

### 📋 Audit / L2
- Added `AuditEvent::FixedPointEntered` and `AuditEvent::FixedPointExited` (additive, tagged enum)
- Summary counters updated
- `Kernel::step` now emits these events when L2 enters or exits the fixed-point regime

---

## Migration Guide

### From 0.5.6 → 0.5.7

**No breaking changes** — this is a feature release with full backward compatibility.

#### New Feature Flag
```toml
# Cargo.toml
[dependencies]
omega-cognitive-kernel = { version = "0.5.7", features = ["async"] }
```

#### Async Integration Pattern
```rust
use omega_cognitive_kernel::{
    AsyncObserverAdapter, AsyncExecutorAdapter,
    FileObserver, SandboxedExecutor,
};

// Wrap sync observer for async use
let sync_observer = FileObserver::new("sensor.json");
let async_observer = AsyncObserverAdapter::new(sync_observer);

// Wrap sync executor for async use
let sync_executor = SandboxedExecutor::new(vec!["observe".into()]);
let async_executor: AsyncExecutorAdapter<_> = sync_executor.into();
// or: let async_executor = AsyncExecutorAdapter::new(sync_executor);
```

---

## Full Changelog

See [CHANGELOG.md](CHANGELOG.md) for complete history.

---

## Verification

All CI checks pass:
- ✅ `cargo fmt --all -- --check`
- ✅ `cargo clippy --locked --all-targets -- -D warnings`
- ✅ `cargo test --locked` (79 tests)
- ✅ `cargo test --locked --features async`
- ✅ `cargo test --locked --features gui --bin omega-dashboard`
- ✅ `cargo build --locked --release --features gui --bin omega-dashboard`
- ✅ `cargo build --locked --lib --no-default-features --target wasm32-unknown-unknown`
- ✅ `cargo build --locked --lib --no-default-features --target wasm32-wasip1`
- ✅ `cargo build --locked --lib --features std --target wasm32-wasip1`
- ✅ `cargo build --locked --example wasm_runtime_smoke --features std --target wasm32-wasip1`

---

## Links

- **Repository**: https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel
- **Crates.io**: https://crates.io/crates/omega-cognitive-kernel
- **Documentation**: https://docs.rs/omega-cognitive-kernel/0.5.7
- **Changelog**: CHANGELOG.md
- **Integrating Guide**: docs/INTEGRATING.md