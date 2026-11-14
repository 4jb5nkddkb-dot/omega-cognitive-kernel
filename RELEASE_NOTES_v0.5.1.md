# Omega Cognitive Kernel v0.5.1

Operator UX + release hardening on the 0.5 line.

## Highlights

- Dashboard **Replay** tab (offline trajectory summary)
- Schema version visible in dashboard status
- Release workflow: GUI binary **required** on Linux & Windows

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.1 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.1 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on 0.5.0. No state schema change.
