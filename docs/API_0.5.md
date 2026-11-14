# API notes — 0.5.x

**Status:** in effect as of **v0.5.0** (extended through **v0.5.4**).

## Additive surface since 0.4

| Item | Version | Change |
|------|---------|--------|
| `KernelStatus.schema_version` | 0.5.0 | New field |
| State migrate missing/0 → 1 | 0.5.0 | Load path |
| `replay_trajectory_file` / CLI `replay` | 0.5.0 | Offline trajectory |
| Dashboard Replay tab | 0.5.1 | Operator UX |
| `KernelEvent` alias | 0.5.2 | `= AuditEvent` |
| `AuditLog::byte_len` / `poll_after` | 0.5.2 | Event tailing |
| `Kernel::set_event_hook` / `clear_event_hook` | 0.5.2 | In-process callback |
| `StrategyMemory::set_weight` / `Kernel::set_strategy_weight` | 0.5.2 | Operator weight nudge |
| Dashboard strategy +/- , multi-workspace | 0.5.3 | Operator UX |
| CLI `strategy-weight` / `audit-poll` | 0.5.4 | Integrator parity |
| Dashboard Audit Follow / poll once | 0.5.4 | Live stream |

## Event stream

```text
KernelEvent = AuditEvent
AuditLog::byte_len() -> u64
AuditLog::poll_after(offset) -> (Vec<AuditRecord>, new_offset)
Kernel::set_event_hook(FnMut(&AuditEvent) + Send + 'static)
Kernel::clear_event_hook()
```

- **In-process:** register a hook; it runs after each audit append inside `step` / related paths.
- **Out-of-process:** poll `audit.jsonl` with a byte cursor (safe for concurrent readers; incomplete trailing lines are held back).

Example: `cargo run --example event_hook_poll`

## Strategy weight

```text
Kernel::set_strategy_weight(name, weight: f64) -> bool   // false if unknown
# weight typically clamped in [0.1, 10]
```

CLI: `omega-kernel strategy-weight <name> <weight>`

## CLI additions (0.5.4)

```bash
omega-kernel strategy-weight explore 1.25
omega-kernel audit-poll --offset 0          # prints new events + next cursor
```

## Still unstable

- `kernel.inner.*` layout
- L1/L2 internal heuristics
- Exact trajectory JSON field set (may grow)
- Exact `AuditEvent` variants may gain fields (serde-tagged; unknown kinds should be ignored by consumers)
