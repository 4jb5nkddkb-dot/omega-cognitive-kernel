# Omega Dashboard

Desktop control panel for the cognitive kernel (egui).

## Build & run

```bash
cargo run --release --features gui --bin omega-dashboard
```

Windows: produces a pure GUI window (no console) via `windows_subsystem`.

## Tabs

| Tab | Purpose |
|-----|---------|
| 仪表盘 | steps / error / damping / strategies / self_modify / L2 |
| 运行 | set goal + step count, run batch, live log |
| 审计 | summary + recent audit events |
| 策略 | strategy memory ranked by weight |
| 设置 | self_modify / dump_trajectory toggles, load state/config |

## Safety defaults

- `self_modify_enabled = false`
- `dump_trajectory = true`
- Local state under temp dir until you open/save another path

## Relation to CLI

Dashboard drives the same `Kernel` API as `omega-kernel`. Prefer CLI for scripting; use Dashboard for operator visibility while developing the closed loop.
