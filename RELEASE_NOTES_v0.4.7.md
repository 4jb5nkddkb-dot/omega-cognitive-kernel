# Omega Cognitive Kernel v0.4.7

Patch release: **dashboard compiles on CI**, docs and version strings aligned.

Prefer this tag over **v0.4.6** if you need `omega-dashboard` binaries.

---

## Artifacts

| File | Platform |
|------|----------|
| `omega-kernel-*-x86_64(.exe)` | Linux / Windows |
| `omega-audit-panel-*-x86_64(.exe)` | Linux / Windows |
| `omega-dashboard-*-x86_64(.exe)` | Linux / Windows (GUI) |

---

## Fixes vs 0.4.6

- `ui_audit` match arms: `Err` branch no longer returns `Response` vs `()` — GUI build succeeds
- README / PRODUCT / ARCHITECTURE version headers updated to **0.4.7**
- CHANGELOG notes that **0.4.5** was never tagged publicly (features live in 0.4.6+)

---

## Still true from 0.4.5–0.4.6

- `self_modify_enabled = false` by default
- `dump_trajectory` → `trajectory.jsonl`
- Desktop dashboard tabs: status / run / audit / strategies / settings

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.7 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.7 --locked --features gui --bin omega-dashboard
```

---

## Compatibility

Additive on the **0.4.x** freeze line. Integrator surface from v0.4.0 unchanged.
