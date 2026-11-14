# Omega Cognitive Kernel v0.5.3

Dashboard operator UX: strategy weight nudge + multi-workspace.

## Highlights

- **Strategies tab:** `+` / `−` / reset-to-1.0 weight controls (calls `Kernel::set_strategy_weight`)
- **Settings → Workspaces:** open / new / switch among local `state.json` instances without restart
- `self_modify` remains **off** by default

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.3 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.3 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on 0.5.2. No state schema change.
