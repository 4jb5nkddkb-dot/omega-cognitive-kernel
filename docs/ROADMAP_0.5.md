# Roadmap — Omega Cognitive Kernel 0.5

**Status:** thin **v0.5.0** shipped (schema migrate + trajectory replay).

## Positioning

0.4.x froze the integrator surface. 0.5 allows controlled breaks for:

1. Explicit state schema versioning + migration
2. Offline trajectory replay
3. Clearer operator status (`schema_version` on `KernelStatus`)

## Shipped in thin 0.5.0

- [x] `CURRENT_SCHEMA_VERSION = 1` with load-time migrate (0 / missing → 1)
- [x] Reject future schema versions with a clear error
- [x] `KernelStatus.schema_version`
- [x] `replay_trajectory_file` + `Kernel::replay_trajectory`
- [x] CLI `omega-kernel replay [--path …] [--tail N]`
- [x] Docs: this roadmap + `API_0.5.md`

## Deferred (0.5.1+)

- [ ] Dashboard operator UX rewrite (self_modify + audit center)
- [ ] Full `KernelEvent` subscribe API (today: `AuditEvent` remains the stream)
- [ ] Linux GUI hard-fail in release workflow (still `continue-on-error`)
- [ ] Multi-instance / workspace layout

## Success bar for 0.5.0

- 0.4 state files load under 0.5
- CI green
- End-to-end: run → trajectory → `replay`
