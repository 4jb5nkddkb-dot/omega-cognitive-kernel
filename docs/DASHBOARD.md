# Omega Dashboard

Desktop control panel for the cognitive kernel (egui).

## Build & run

```bash
cargo run --release --features gui --bin omega-dashboard
```

Or install from this checkout (run from the repository root):

```bash
cargo install --path . --locked --features gui --bin omega-dashboard
```

Windows: produces a pure GUI window (no console) via `windows_subsystem`.

## Quick demo flow (recommended)

1. Start the dashboard.
2. Go to **Settings** and confirm `self_modify_enabled` is off (default).
3. Switch to **Run** tab, set a goal (e.g. "stabilize under load"), choose step count, and run.
4. Watch **Dashboard** tab for live error / damping / strategy weights.
5. Check **Audit** for events and **Replay** for trajectory summary.
6. To inspect state from another process or an example, choose **Settings → Open state** and select its `state.json`.

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
- The dashboard starts with state under the system temp directory (`omega-dashboard-state.json`); it does not automatically discover example or CLI state files.
- Use **Settings → Open state** to load another state file. Audit and trajectory views read their sibling files beside that state.

## Relation to CLI & examples

Dashboard drives the same `Kernel` API as `omega-kernel`. Prefer CLI for scripting; use Dashboard for operator visibility while developing the closed loop.

Pair with the drop-inbox demo:

```bash
cargo run --example drop_inbox_loop
# In the dashboard, choose Settings > Open state and select:
# .omega/drop-inbox-demo/state.json
```

The example keeps its data in `.omega/drop-inbox-demo/` and resumes the saved
snapshot on subsequent runs. Set `OMEGA_DEMO_DIR` to relocate that directory;
use the resulting `<OMEGA_DEMO_DIR>/state.json` path in **Open state**.

See also [PRODUCT.md](./PRODUCT.md) for the overall narrative.
