# API notes — 0.5.x

**Status:** in effect as of **v0.5.0** (extended in 0.5.1 / 0.5.2).

## Additive surface since 0.4

| Item | Version | Change |
|------|---------|--------|
| `KernelStatus.schema_version` | 0.5.0 | New field |
| State migrate missing/0 → 1 | 0.5.0 | Load path |
| `replay_trajectory_file` / CLI `replay` | 0.5.0 | Offline trajectory |
| Dashboard Replay tab | 0.5.1 | Operator UX |
| `KernelEvent` alias | 0.5.2 | `= AuditEvent` |
| `AuditLog::byte_len` / `poll_after` | 0.5.2 | Event tailing |

## Event stream

```text
KernelEvent = AuditEvent
AuditLog::poll_after(offset) -> (Vec<AuditRecord>, new_offset)
```

No in-process callback bus yet — poll `audit.jsonl` (or read after `step`).

## Still unstable

- `kernel.inner.*` layout
- L1/L2 internal heuristics
- Exact trajectory JSON field set (may grow)
