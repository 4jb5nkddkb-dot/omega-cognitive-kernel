# Tool-call gate (safety MVP)

**Command**

```bash
cargo run --example tool_gate
```

## Story

1. A host/agent (here: scripted list) **proposes** tool names.
2. `ToolGateExecutor` enforces **allow-list** + **deny-list**.
3. Allowed tools return simulated `ok` (no real I/O in this type).
4. Denied tools never run; `ToolDenied` / `ToolAllowed` land in `audit.jsonl`.

## What this is / is not

| Is | Is not |
|----|--------|
| Last-line gate before side effects | Full LLM HTTP proxy |
| Auditable allow/deny | Content NLP classifier |
| Building block for compliance narrative | Production multi-tenant gateway |

## Library

```rust
use omega_cognitive_kernel::{AuditLog, ToolGateExecutor, Executor};

let mut gate = ToolGateExecutor::new(["search_docs".into()])
    .deny(["delete_database".into()])
    .with_audit(AuditLog::beside_state(path))
    .fail_closed(true);
gate.execute(&proposed_actions)?;
```

## Roadmap

1. [x] `ToolGateExecutor` + example + audit events
2. [ ] Optional real `http_get` only after ALLOW
3. [ ] Host service wrapping gate + kernel status

See [PRODUCT.md](./PRODUCT.md), [INTERVENTION_POLICY.md](./INTERVENTION_POLICY.md).
