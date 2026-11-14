# Omega Cognitive Kernel v0.4.9

Stability release on the 0.4.x freeze line.

## Highlights

- Case-memory prioritized sampling made CI-stable (counter-seeded LCG + test threshold)
- Orchestrator high-error patch test asserts accepted patch + damping (no flaky error trajectory)
- CI hardening: strict fmt/clippy; scheduled autofix opens PRs instead of pushing to main
- Version / docs alignment

## Still true from 0.4.8

- Case memory: priority ∝ `|error_mean|^α`, evict lowest priority, IS weights helper
- `self_modify_enabled = false` by default
- Desktop dashboard tabs: status / run / audit / strategies / settings

## Artifacts

| File | Platform |
|------|----------|
| `omega-kernel-*-x86_64(.exe)` | Linux / Windows |
| `omega-audit-panel-*-x86_64(.exe)` | Linux / Windows |
| `omega-dashboard-*-x86_64(.exe)` | Linux / Windows (GUI) |

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.9 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.9 --locked --features gui --bin omega-dashboard
```

## Compatibility

Additive on the **0.4.x** freeze line. Integrator surface from v0.4.0 unchanged.
