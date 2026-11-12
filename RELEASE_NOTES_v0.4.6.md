# Omega Cognitive Kernel v0.4.6

Catch-up release on the **0.4.x** line: safety switches, trajectory dump, audit panel binary, and an optional **desktop dashboard**.

Does **not** break the v0.4.0 integrator freeze surface.

---

## Artifacts (this Release)

| File | Platform | Notes |
|------|----------|--------|
| `omega-kernel-linux-x86_64` | Linux | Main CLI |
| `omega-audit-panel-linux-x86_64` | Linux | Audit operator |
| `omega-dashboard-linux-x86_64` | Linux | GUI (if build ok) |
| `omega-kernel-windows-x86_64.exe` | Windows | Main CLI |
| `omega-audit-panel-windows-x86_64.exe` | Windows | Audit operator |
| `omega-dashboard-windows-x86_64.exe` | Windows | GUI (if build ok) |

Linux pack may also ship as `omega-kernel-linux-x86_64.tar.gz`.

---

## Highlights

### Self-modify master switch (0.4.5+)

```toml
# omega.toml
self_modify_enabled = false   # default — L1 will not commit patches
dump_trajectory = true        # append trajectory.jsonl beside state
```

- Hard gate inside `L1CognitiveLayer::process`
- Visible in `omega-kernel status` and run summary

### Trajectory + closed-loop docs

- Every `Kernel::step` can append a JSONL record
- `docs/CLOSED_LOOP.md` checklist

### Desktop dashboard (0.4.6)

```bash
cargo run --release --features gui --bin omega-dashboard
# or download omega-dashboard-*-x86_64(.exe) from this Release
```

Operator UI: status cards, batch run, audit, strategy table, toggles for self_modify / dump.

### Other binaries

```bash
omega-kernel status
omega-audit-panel
```

### CI

- Weekly runtime report → `reports/weekly-YYYY-Www.md`
- Release builds CLI + GUI on Linux and Windows

---

## Install from source

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.6 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.6 --locked --features gui --bin omega-dashboard
```

---

## Compatibility

- Additive relative to v0.4.4
- Default behaviour: **no self-modification** until config enables it
- Integrator freeze from v0.4.0 remains in effect
