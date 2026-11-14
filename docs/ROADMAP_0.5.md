# Roadmap — Omega Cognitive Kernel 0.5

**Status:** **v0.5.1** shipped; **0.5.2** event-stream helpers on main.

## Shipped

### 0.5.0
- [x] Schema version + migrate
- [x] Trajectory replay API + CLI
- [x] `KernelStatus.schema_version`
- [x] Docs: ROADMAP / API_0.5

### 0.5.1
- [x] Dashboard Replay tab
- [x] Dashboard schema_version display
- [x] Release: hard-require GUI artifacts (Linux + Windows)

### 0.5.2 (main)
- [x] `KernelEvent` = `AuditEvent` public alias
- [x] `AuditLog::byte_len` / `poll_after` for integrator tailing
- [x] PRODUCT / INTEGRATING aligned to 0.5.x

## Still deferred

- [ ] In-process callback sinks on `Kernel::step` (today: poll audit.jsonl)
- [ ] Multi-instance / workspace layout
- [ ] Deeper dashboard strategy editing
