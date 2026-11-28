# Persistence privacy and file permissions

## What is recorded by default

For a state path such as `.omega/state.json`, the kernel stores the following files beside it:

| Path | Default behavior | Typical contents |
|---|---|---|
| `state.json` | Autosaved after a successful step (`autosave = true`) | Kernel counters, numeric policy and learning state, case summaries, and the goal text retained with each case |
| `audit.jsonl` | Appended during kernel operations | Step events, task-episode events, checkpoint/rollback events, and other audit events; step goals and caller-supplied task/checkpoint labels may be present |
| `trajectory.jsonl` | Appended for each step (`dump_trajectory = true`) up to 64 MiB by default | Step number, goal, action label, prediction-error and L1/L2 metrics |
| `checkpoints/` | Created when requested and automatically before an accepted patch (`auto_checkpoint_on_patch = true`) | Snapshot files and checkpoint metadata, including the supplied label |
| `<state-file>.step-journal/<step-id>.jsonl` | Appended for each step | Step ID, goal, attempt number, and executor/commit status used to prevent unsafe replay |

An explicit `state_path` changes the state file location; the audit, trajectory, checkpoint, and step-journal paths remain beside that state file. The default 64 MiB trajectory limit can be changed per kernel with `set_trajectory_max_bytes` in Rust or Python. A row that would exceed the limit is rejected and the failure propagates through the step; existing rows are never truncated or rotated and remain replayable. Set `dump_trajectory = false` to stop trajectory appends, `autosave = false` to stop automatic state saves, or `auto_checkpoint_on_patch = false` to stop automatic pre-patch checkpoints. Manual `save()` and `checkpoint()` calls still write their respective files.

## Opt-in redaction

Older TOML files remain valid and keep the legacy behavior. To redact known user-provided text in kernel-owned persistence writes, add:

```toml
[privacy]
redact_user_content = true
```

The default is `false`. When enabled, the kernel writes `[REDACTED]` in place of step goals in `audit.jsonl`, task names passed to `audit_task`, checkpoint labels, case goals in state and checkpoint snapshots, and trajectory goals. The step sidecar journal also stores `[REDACTED]` plus a SHA-256 goal digest so stable-ID retry checks continue to work; this digest is a matching fingerprint, not encryption, and may be guessable for predictable goals. `Kernel::snapshot_json()` uses the same redaction. Numeric state and the state snapshot schema are unchanged; older journal rows remain readable. `Kernel::snapshot()` is an in-memory API and continues to return the live, unredacted snapshot. In-process event hooks also receive the original event so applications can make their own handling choice.

This is targeted redaction, not an automatic secret or PII detector, and it is not encryption. It does not rewrite earlier audit, trajectory, or step-journal rows, old checkpoint files, or copies written directly by an integrating application. Existing append-only history remains as-is; remove or rotate old files yourself if they already contain text you want deleted. Direct calls to `AuditLog::append` or `CheckpointStore::save` outside the configured `Kernel` also retain their existing unredacted behavior.

## File and directory permissions

When the kernel creates a new persistence directory on Unix, including a step-journal directory, it requests mode `0700`; newly created state, audit, trajectory, checkpoint snapshot, checkpoint metadata, and step-journal files request mode `0600`. The OS applies its normal permission rules (including the process umask). Existing directories are not chmod'd. Existing audit, trajectory, checkpoint, and step-journal files are not chmod'd when appended or read. State saving uses a same-directory temporary file and atomic rename, so the replacement state file receives the new-file mode on the next save.

Windows does not expose POSIX mode bits through these APIs. New files and directories use the current user's default ACL and inheritance; this feature does not rewrite existing ACLs. For sensitive deployments, also protect the parent directory and backups according to the host's operating-system policy.
