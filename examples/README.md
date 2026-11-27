# Examples

Runnable demos that show how to embed or observe the kernel.

## Quick start (recommended path)

```bash
# Closed-loop product story: drop files into inbox, watch policy adapt
cargo run --example drop_inbox_loop

# Data stays in .omega/drop-inbox-demo; rerunning resumes the saved state.
# Then inspect that state with the desktop UI.
cargo run --release --features gui --bin omega-dashboard
```
In the dashboard, choose **Settings → Open state** and select `.omega/drop-inbox-demo/state.json`. Set `OMEGA_DEMO_DIR` before running the example to use a different directory. Remove the demo directory when you want to start over.

## Available examples

| Example | What it shows |
|---------|---------------|
| `drop_inbox_loop` | End-to-end closed loop with `DirSnapshotObserver` + `LoggingExecutor`; saves and resumes state (primary demo) |
| `event_hook_poll` | In-process event hook + file-tail polling of the audit log |
| `audit_http_shim` | Minimal HTTP `/status` endpoint |
| `config_and_dir` | `omega.toml` + directory observer |
| `custom_adapters` | Implementing your own `Observer` / `Executor` |
| `tool_gate` / `tool_gate_http` | Sandboxed tool execution with allow-list |
| `inbox_vertical` | Longer vertical scenario |
| `async_observer_demo` | **Async** file/dir/HTTP observers using tokio (requires `--features async`) |
| `async_executor_demo` | **Async** executors & logging using tokio (requires `--features async`) |
| `l1_probe` | L1 shadow zero-perturbation, L1/L2 write-mutex arbitration, and rollback-window probe (`cargo run --release --example l1_probe`) |
| `audit_panel` | Minimal audit panel integration example |
| `wasm_runtime_smoke` | WASM runtime smoke check (`wasm32-wasip1` / `wasm32-unknown-unknown`) |

All examples respect the safety default `self_modify_enabled = false`.

See [docs/INTEGRATING.md](../docs/INTEGRATING.md) and [docs/PRODUCT.md](../docs/PRODUCT.md) for more context.
