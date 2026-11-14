# Omega Cognitive Kernel v0.5.5

Operator status export: CLI JSON + HTTP shim parity.

## Highlights

- CLI: `omega-kernel status --json` → pretty `KernelStatus`
- Example: `audit_http_shim` `GET /status` (localhost, read-only)
- Docs/examples from post-0.5.4 (event_hook_poll, NOTICE, crates.io checklist)

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.5 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.5 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on 0.5.4. No state schema change.
