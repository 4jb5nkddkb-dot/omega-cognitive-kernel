# Omega Dashboard

Desktop control panel for the cognitive kernel (egui).

## Build & run

```bash
cargo run --release --features gui --bin omega-dashboard
```

Or install the binary:

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.6 --locked --features gui --bin omega-dashboard
```

Windows: produces a pure GUI window (no console) via `windows_subsystem`.

## Quick demo flow (recommended)

1. Start the dashboard.
2. Go to **Settings** and confirm `self_modify_enabled` is off (default).
3. Switch to **Run** tab, set a goal (e.g. "stabilize under load"), choose step count, and run.
4. Watch **Dashboard** tab for live error / damping / strategy weights.
5. Check **Audit** for events and **Replay** for trajectory summary.
6. Optionally open a saved `state.json` from a previous CLI or example run.

## Tabs

| Tab | Purpose |
|-----|---------|
| Dashboard (仪表盘) | steps / error / damping / strategies / self_modify / L2 |
| Run (运行) | set goal + step count, run batch, live log |
| Audit (审计) | summary + recent audit events (live follow) |
| Strategies (策略) | strategy memory ranked by weight; nudge controls |
| Workspaces | multi-workspace status |
| Replay | trajectory rows / patches summary |
| Settings (设置) | self_modify / dump_trajectory toggles, load state/config |

## Safety defaults

- `self_modify_enabled = false`
- `dump_trajectory = true`
- Local state under temp dir until you open/save another path

## Relation to CLI & examples

Dashboard drives the same `Kernel` API as `omega-kernel`. Prefer CLI for scripting; use Dashboard for operator visibility while developing the closed loop.

Pair with the drop-inbox demo:

```bash
cargo run --example drop_inbox_loop
# then open the generated state.json in the dashboard if desired
```

See also [PRODUCT.md](./PRODUCT.md) for the overall narrative.
