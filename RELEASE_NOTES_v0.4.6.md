# Omega Cognitive Kernel v0.4.6

Catch-up release on the **0.4.x** line: safety switches, trajectory dump, audit panel binary, and an optional **desktop dashboard**.

Does **not** break the v0.4.0 integrator freeze surface.

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

- Every `Kernel::step` can append a JSONL record (goal, action, errors, patch flags, L2)
- `docs/CLOSED_LOOP.md` checklist

### Desktop dashboard (0.4.6)

```bash
cargo run --release --features gui --bin omega-dashboard
```

Operator UI: status cards, batch run, audit, strategy table, toggles for self_modify / dump.

### Other binaries

```bash
cargo run --release --bin omega-kernel -- status
cargo run --release --bin omega-audit-panel
```

### CI

- Weekly runtime report workflow → `reports/weekly-YYYY-Www.md`

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.6 --locked

# optional GUI
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.6 --locked --features gui --bin omega-dashboard
```

---

## Compatibility

- Additive relative to v0.4.4
- Default behaviour: **no self-modification** until config enables it
- Integrator freeze from v0.4.0 remains in effect
