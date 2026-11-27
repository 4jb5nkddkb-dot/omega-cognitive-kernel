# Omega Cognitive Kernel v0.4.3

Patch on the **0.4 freeze line**: tool-gate can perform a **real read-only HTTP GET** after ALLOW.

---

## What's new

```bash
cargo run --example tool_gate_http
```

- `ToolGateExecutor::enable_http_get(true)`
- Tool name `http_get` + `params.url` → GET with timeouts and truncated body
- Deny-list tools never open network connections
- Still **not** a full LLM proxy or POST/shell gateway

See [docs/TOOL_GATE.md](./docs/TOOL_GATE.md).

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.3 --locked
```

---

## Compatibility

Additive only relative to v0.4.2. Integrator freeze from v0.4.0 remains in effect.
