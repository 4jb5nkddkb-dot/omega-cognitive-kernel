# Persistence privacy and file permissions

## What is recorded by default

For a state path such as `.omega/state.json`, the kernel stores the following files beside it:

| Path | Default behavior | Typical contents |
|---|---|---|
| `state.json` | Autosaved after a successful step (`autosave = true`) | Kernel counters, numeric policy and learning state, case summaries, and the goal text retained with each case |
| `audit.jsonl` | Appended during kernel operations | Step events, task-episode events, checkpoint/rollback events, and other audit events; step goals and caller-supplied task/checkpoint labels may be present |
| `trajectory.jsonl` | Appended for each step (`dump_trajectory = true`) | Step number, goal, action label, prediction-error and L1/L2 metrics |
| `checkpoints/` | Created when requested and automatically before an accepted patch (`auto_checkpoint_on_patch = true`) | Snapshot files and checkpoint metadata, including the supplied label |

An explicit `state_path` changes the state file location; the audit, trajectory, and checkpoint paths remain siblings of that state file. Set `dump_trajectory = false` to stop trajectory appends, `autosave = false` to stop automatic state saves, or `auto_checkpoint_on_patch = false` to stop automatic pre-patch checkpoints. Manual `save()` and `checkpoint()` calls still write their respective files.

## Opt-in redaction

Older TOML files remain valid and keep the legacy behavior. To redact known user-provided text in kernel-owned persistence writes, add:

```toml
[privacy]
redact_user_content = true
```

The default is `false`. When enabled, the kernel writes `[REDACTED]` in place of step goals in `audit.jsonl`, task names passed to `audit_task`, checkpoint labels, case goals in state and checkpoint snapshots, and trajectory goals. `Kernel::snapshot_json()` uses the same redaction. Numeric state and the on-disk schema are unchanged; no schema migration is needed. `Kernel::snapshot()` is an in-memory API and continues to return the live, unredacted snapshot. In-process event hooks also receive the original event so applications can make their own handling choice.

This is targeted redaction, not an automatic secret or PII detector, and it is not encryption. It does not rewrite earlier audit or trajectory rows, old checkpoint files, or copies written directly by an integrating application. Existing append-only history remains as-is; remove or rotate old files yourself if they already contain text you want deleted. Direct calls to `AuditLog::append` or `CheckpointStore::save` outside the configured `Kernel` also retain their existing unredacted behavior.

## File and directory permissions

When the kernel creates a new persistence directory on Unix, it requests mode `0700`; newly created state, audit, trajectory, checkpoint snapshot, and checkpoint metadata files request mode `0600`. The OS applies its normal permission rules (including the process umask). Existing directories are not chmod'd. Existing audit, trajectory, and checkpoint files are not chmod'd when appended or read. State saving uses a same-directory temporary file and atomic rename, so the replacement state file receives the new-file mode on the next save.

Windows does not expose POSIX mode bits through these APIs. New files and directories use the current user's default ACL and inheritance; this feature does not rewrite existing ACLs. For sensitive deployments, also protect the parent directory and backups according to the host's operating-system policy.
