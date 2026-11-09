# Intervention policy (strategy governance)

**Problem:** assistants often stay trapped in the user's local frame ("fix dlltool") instead of proposing a lower-cost path (Codespaces) — or, worse, over-advise on high-stakes domains.

**Response:** explicit **mode + friction + max intervention level**, stored in `omega.toml`, visible to operators via config and (later) audit panel.

This is **policy configuration**, not model weights. Runtime enforcement can grow in 0.5.x; the contract starts here.

---

## Minimal `omega.toml` section

```toml
[intervention]
mode = "exploration"          # or "strict"
max_level = "suggest_alternative"  # silent | in_frame | suggest_alternative | refuse_to_decide
friction_threshold = 2        # consecutive env/build failures before alternative path
audit_escalations = true
```

Full sample: `examples/omega.intervention.example.toml`.

---

## Semantics

| Field | Meaning |
|-------|---------|
| `mode = exploration` | DX / tooling; **may** escalate to path alternatives after friction |
| `mode = strict` | High-stakes; effective cap becomes **refuse_to_decide** (no auto path-substitute-as-decision) |
| `friction_threshold` | Streak of similar failures before `may_suggest_alternative` is true |
| `max_level` | Ceiling on how far the system may go |
| `audit_escalations` | Prefer recording allow/suppress decisions in audit (when wired) |

Library helpers (already in `InterventionPolicy`):

- `effective_max_level()` — applies strict-mode clamp
- `may_suggest_alternative(consecutive_failures)` — friction gate

---

## Risk spectrum (operator rule of thumb)

| Domain | Recommended mode |
|--------|------------------|
| Local toolchain, CI, Codespaces, UI layout | `exploration` + `suggest_alternative` |
| Product demo / inbox vertical | `exploration` |
| Medical, legal, finance, safety-critical ops | `strict` + `refuse_to_decide` |

**Principle:** low cost of being wrong → allow surprise alternatives; high cost of being wrong → stay in-frame or refuse to decide.

---

## 0.4 compatibility

- New fields are **additive** with `serde` defaults
- Existing `omega.toml` without `[intervention]` keeps prior behavior
- No break of the 0.4 freeze list signatures

---

## Roadmap (not all shipped)

1. [x] Config types + example TOML + this doc
2. [ ] Wire friction counters from host/agent observations
3. [ ] Audit events: `path_alternative_suggested` / `intervention_suppressed`
4. [ ] Surface policy block in `audit --summary`

See also: [AUDIT_PANEL.md](./AUDIT_PANEL.md), [PRODUCT.md](./PRODUCT.md).
