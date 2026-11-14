# Roadmap — Omega Cognitive Kernel 0.5

**Status:** **v0.5.4** + post-release docs/examples (no version bump).

## Shipped

### 0.5.0–0.5.3
- Schema, replay, event hooks, strategy weights, dashboard workspaces

### 0.5.4
- [x] CLI `strategy-weight` / `audit-poll`
- [x] Dashboard Audit live follow (`poll_after`)

### Post-0.5.4 (no version bump)
- [x] `examples/event_hook_poll.rs` — in-process hook + file `poll_after`
- [x] `docs/INTEGRATING.md` / `API_0.5.md` aligned to 0.5.4
- [x] `NOTICE` + open-core / trademark posture
- [x] `examples/audit_http_shim` — `GET /status` (read-only `KernelStatus`)
- [x] `RELEASING.md` crates.io checklist (publish when deliberately ready)

## Still deferred

- [ ] Full multi-tenant control plane
- [ ] Workspace multi-kernel orchestration service
- [ ] Actual crates.io publish (checklist only until then)
