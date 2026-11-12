# Changelog

## [0.4.6] — 2026-11-12

### Desktop dashboard

- Optional feature `gui` + binary `omega-dashboard` (egui)
- Tabs: 仪表盘 / 运行 / 审计 / 策略 / 设置
- Windows: no console window; CJK font load
- Docs: `docs/DASHBOARD.md`

### Build

```bash
cargo run --release --features gui --bin omega-dashboard
```

## [0.4.5] — 2026-11-12

### Safety & observability

- `KernelConfig.self_modify_enabled` (default **false**) — hard gate in L1
- `KernelConfig.dump_trajectory` (default true) → `trajectory.jsonl` beside state
- CLI `status` / `run` print `self_modify_enabled` and `dump_trajectory`
- Standalone `omega-audit-panel` binary
- Weekly runtime report workflow → `reports/weekly-*.md`
- Docs: `docs/CLOSED_LOOP.md`

## [0.4.4] — 2026-11-10

### Audit HTTP shim (demo)

- `examples/audit_http_shim.rs` — localhost read-only HTTP over `audit.jsonl`
- Routes: `GET /health`, `/audit/summary`, `/audit/tail?n=`
- Bind `127.0.0.1` only; no TLS/auth — not a production gateway
- Doc: `docs/AUDIT_HTTP_SHIM.md`

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
