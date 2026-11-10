# Changelog

## [0.4.3] — 2026-11-10

### Tool gate + real HTTP GET

- `ToolGateExecutor::enable_http_get(true)` — after ALLOW, tool `http_get` may perform a real GET
- Constraints: `http://` / `https://` only; connect/read timeouts; body preview ≤ 2 KiB
- DENY paths still open **no** sockets
- Example: `examples/tool_gate_http.rs`
- Doc: `docs/TOOL_GATE.md` updated

Additive on 0.4.x; freeze surface unchanged.

## [0.4.2] — 2026-11-10

### Tool-call gate (safety MVP)

- `ToolGateExecutor` — allow-list + deny-list, optional audit, `fail_closed`
- Audit events: `ToolAllowed`, `ToolDenied`
- `AuditSummary.tools_allowed` / `tools_denied`
- Example: `examples/tool_gate.rs`
- Doc: `docs/TOOL_GATE.md`

### Audit panel & intervention policy

- CLI: `omega-kernel audit --summary`
- `AuditSummary` / `AuditLog::summarize`
- `[intervention]` in `omega.toml`
- Docs: `docs/AUDIT_PANEL.md`, `docs/INTERVENTION_POLICY.md`

## [0.4.1] — 2026-11-09

- Inbox vertical demo + README links

## [0.4.0] — 2026-11-09

- Integrator API freeze for 0.4.x

## [0.3.9] — [0.1.0]

- See prior tags
