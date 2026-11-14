# Omega Cognitive Kernel v0.5.0

Thin 0.5 release: **explicit state schema + offline trajectory replay**.

## Highlights

- `state.json` schema version stamped (`CURRENT_SCHEMA_VERSION = 1`)
- Load-time migration from pre-0.5 snapshots (missing version → 1)
- Reject unknown future schema versions with a clear error
- `KernelStatus.schema_version`
- CLI: `omega-kernel replay [--path trajectory.jsonl] [--tail 20]`
- Library: `replay_trajectory_file`, `Kernel::replay_trajectory`

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.0 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.0 --locked --features gui --bin omega-dashboard
```

## Quick check

```bash
omega-kernel run --steps 8
omega-kernel replay --tail 10
omega-kernel status
```

## Compatibility

- 0.4 state files load under 0.5
- Additive `KernelStatus` field; see `docs/API_0.5.md`
- Dashboard / full event subscribe deferred to 0.5.1+
