# Changelog

## [0.4.4] — 2026-11-10

### Audit HTTP shim (demo)

- `examples/audit_http_shim.rs` — localhost read-only HTTP over `audit.jsonl`
- Routes: `GET /health`, `/audit/summary`, `/audit/tail?n=`
- Bind `127.0.0.1` only; no TLS/auth — not a production gateway
- Doc: `docs/AUDIT_HTTP_SHIM.md`
- README links the shim

Library freeze surface unchanged (example + docs).

## [0.4.3] — 2026-11-10

- `ToolGateExecutor::enable_http_get` — real GET after ALLOW
- Example `tool_gate_http`

## [0.4.2] — 2026-11-10

- Tool gate allow/deny + audit events
- `audit --summary`, `[intervention]` policy

## [0.4.1] — 2026-11-09

- Inbox vertical demo

## [0.4.0] — 2026-11-09

- Integrator API freeze for 0.4.x

## Earlier

- See prior tags (0.3.x …)
