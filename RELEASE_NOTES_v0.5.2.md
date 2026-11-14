# Omega Cognitive Kernel v0.5.2

Event stream helpers + operator strategy nudge.

## Highlights

- `KernelEvent` alias (`= AuditEvent`)
- `AuditLog::poll_after` / `byte_len` for incremental tailing
- In-process `Kernel::set_event_hook` (called on each audit event)
- `StrategyMemory::set_weight` for operator weight nudge
- Dashboard: strategy weight +/- and recent workspace states

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.2 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.2 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on 0.5.1.
