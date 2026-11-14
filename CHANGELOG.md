# Changelog

## [0.5.1] — 2026-11-14

### Dashboard

- New **Replay** tab: offline `trajectory.jsonl` summary (rows, patches, action histogram, tail)
- Dashboard shows `schema_version` + self_modify status consistently
- Footer version aligned to 0.5.1

### Release

- GUI build is **required** on Linux and Windows (no `continue-on-error`)
- Linux tarball always includes `omega-dashboard-linux-x86_64`

## [0.5.0] — 2026-11-14

### Thin 0.5.0 — schema + replay

- **State schema:** `CURRENT_SCHEMA_VERSION = 1`; load migrates missing/0 → 1; rejects newer versions
- **KernelStatus:** additive field `schema_version`
- **Replay:** `replay_trajectory_file` / `Kernel::replay_trajectory` + CLI `omega-kernel replay`
- **Docs:** `docs/ROADMAP_0.5.md`, `docs/API_0.5.md`

## [0.4.9] — 2026-11-14

### Stability

- Case memory sampling + orchestrator patch test de-flaked
- CI: strict fmt/clippy; scheduled quality workflow opens PRs

## [0.4.8] — 2026-11-12

### Case memory: prioritized experience replay

- Priority ∝ `|error_mean|^α`; evict lowest priority; IS weights helper

## Earlier

- See tags 0.4.0–0.4.7 and prior
