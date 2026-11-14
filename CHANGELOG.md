# Changelog

## [Unreleased]

### Docs / examples
- `examples/event_hook_poll.rs` — in-process `set_event_hook` + `AuditLog::poll_after`
- `examples/audit_http_shim` — `GET /status` (read-only `KernelStatus`)
- `docs/INTEGRATING.md`, `docs/API_0.5.md` aligned to v0.5.4
- `NOTICE` trademark / open-core boundary; README + PRODUCT posture
- `RELEASING.md` crates.io publish checklist

## [0.5.4] — 2026-11-14

### CLI
- `strategy-weight <name> <weight>` operator nudge
- `audit-poll --offset N` incremental audit via `poll_after`

### Dashboard
- Audit **Follow** / Poll once / Reset cursor (live stream)

## [0.5.3] — 2026-11-14

### Dashboard
- Strategy weight operator nudge (`+` / `−` / reset to 1.0)
- Multi-workspace switcher (open / new / recent state.json instances)

## [0.5.2] — 2026-11-14

### Events
- `KernelEvent`, `poll_after` / `byte_len`, `set_event_hook`

### Strategies
- `set_strategy_weight`

## [0.5.1] — 2026-11-14
- Dashboard Replay tab; required GUI release artifacts

## [0.5.0] — 2026-11-14
- Schema migrate + trajectory replay

## Earlier
- See tags 0.4.x–0.5.x
