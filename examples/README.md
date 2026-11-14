# Examples

Runnable demos that show how to embed or observe the kernel.

## Quick start (recommended path)

```bash
# Closed-loop product story: drop files into inbox, watch policy adapt
cargo run --example drop_inbox_loop

# Then inspect with the desktop UI
cargo run --release --features gui --bin omega-dashboard
```

## Available examples

| Example | What it shows |
|---------|---------------|
| `drop_inbox_loop` | End-to-end closed loop with `DirSnapshotObserver` + `LoggingExecutor` (primary demo) |
| `event_hook_poll` | In-process event hook + file-tail polling of the audit log |
| `audit_http_shim` | Minimal HTTP `/status` endpoint |
| `config_and_dir` | `omega.toml` + directory observer |
| `custom_adapters` | Implementing your own `Observer` / `Executor` |
| `tool_gate` / `tool_gate_http` | Sandboxed tool execution with allow-list |
| `inbox_vertical` | Longer vertical scenario |

All examples respect the safety default `self_modify_enabled = false`.

See [docs/INTEGRATING.md](../docs/INTEGRATING.md) and [docs/PRODUCT.md](../docs/PRODUCT.md) for more context.
