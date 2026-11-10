# Tool-call gate (safety MVP)

**Commands**

```bash
cargo run --example tool_gate
# with real GET after ALLOW:
cargo run --example tool_gate_http
```

## Story

1. Host/agent **proposes** tool names (+ optional `params`).
2. `ToolGateExecutor` enforces **allow-list** + **deny-list**.
3. Denied tools **never** run (no network).
4. Allowed tools are audited (`ToolAllowed` / `ToolDenied`).
5. Optional: `enable_http_get(true)` — only tool `http_get` may perform a **GET** (`params.url`), response body truncated.

## What this is / is not

| Is | Is not |
|----|--------|
| Last-line gate before side effects | Full LLM HTTP proxy |
| Auditable allow/deny | Content NLP classifier |
| One real read-only tool path | POST/PUT/shell/DB access |

## Library

```rust
use omega_cognitive_kernel::{AuditLog, ToolGateExecutor, Executor};

let mut gate = ToolGateExecutor::new(["http_get".into(), "search_docs".into()])
    .deny(["delete_database".into()])
    .with_audit(AuditLog::beside_state(path))
    .enable_http_get(true)  // real GET only for http_get
    .fail_closed(true);
gate.execute(&proposed_actions)?;
```

`http_get` rules: `http://` or `https://` only; connect/read timeouts; body preview ≤ 2 KiB.

## Roadmap

1. [x] Gate + audit events
2. [x] Real `http_get` after ALLOW
3. [ ] Host service wrapping gate + kernel status

See [PRODUCT.md](./PRODUCT.md), [INTERVENTION_POLICY.md](./INTERVENTION_POLICY.md).
