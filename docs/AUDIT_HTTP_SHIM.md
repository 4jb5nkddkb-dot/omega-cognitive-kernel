# Audit HTTP shim (localhost)

Minimal **read-only** HTTP surface over `audit.jsonl` — no new crate dependencies, bind **127.0.0.1** only.

```bash
cargo run --example audit_http_shim
# or point at an existing state file (audit lives beside it):
cargo run --example audit_http_shim -- ./omega-inbox-demo/state.json

# other terminal
curl -s http://127.0.0.1:8787/health
curl -s http://127.0.0.1:8787/audit/summary
curl -s 'http://127.0.0.1:8787/audit/tail?n=10'
```

Port: `OMEGA_AUDIT_PORT` (default `8787`).

## Routes

| Method | Path | Body |
|--------|------|------|
| GET | `/health` | `{ "ok": true, ... }` |
| GET | `/audit/summary` | `AuditSummary` JSON |
| GET | `/audit/tail?n=20` | recent `AuditRecord`s |

## What this is / is not

| Is | Is not |
|----|--------|
| Proof external tools can consume audit | Production API gateway |
| Localhost demo | TLS / auth / multi-tenant |
| std `TcpListener` only | Axum service skeleton |

Next product step (when you have a caller): real bind address + token, or wrap in Axum deliberately.

See [AUDIT_PANEL.md](./AUDIT_PANEL.md), [TOOL_GATE.md](./TOOL_GATE.md).
