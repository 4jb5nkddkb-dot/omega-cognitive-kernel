# Omega Cognitive Kernel v0.5.4

Integrator/operator parity for event stream and strategy nudge.

## Highlights

- CLI: `omega-kernel strategy-weight <name> <weight>`
- CLI: `omega-kernel audit-poll --offset N` (prints new events + next cursor)
- Dashboard Audit: **Follow** / Poll once / Reset cursor

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.4 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.4 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on 0.5.3.
