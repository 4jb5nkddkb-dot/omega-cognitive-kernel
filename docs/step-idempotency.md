# Stable step IDs and recovery

Use `step_with_id` when a step can be retried because a request timed out, the client disconnected, or the process restarted. Derive the ID from the caller's stable request/idempotency key and reuse the **same ID and goal** for every retry:

```rust
let step_id = uuid::Uuid::parse_str(request.idempotency_key)?;
let result = kernel.step_with_id(step_id, goal)?;
```

`step()` is a convenience API that generates a fresh ID for each call. It does not make separate calls idempotent; use `step_with_id` for retryable requests.

Each ID has an append-only JSONL journal beside the configured state file, under `<state-file>.step-journal/<step-id>.jsonl`. The journal records `started`, `dispatching`, `executed`, `not_executed`, and `committed` stages. The executor-dispatch marker is synced before an executor call; the success marker follows a successful return. Configured audit, trajectory, and autosave writes finish before the final commit marker. The optional pre-patch checkpoint is best effort. A step returns success only after the final marker is durable.

## Interpreting errors

- **`StepNotExecuted`**: the executor was not dispatched, and the attempt was marked `not_executed`. Retrying with the same ID and goal is safe; the next attempt is recorded separately.
- **`StepExecutionOutcomeUnknown`**: dispatch may have happened, but successful completion is not durably known (or the journal cannot be trusted). Do not automatically replay it. Reconcile any external effect first.
- **`StepCommitIncomplete`**: the executor returned successfully, but a later record or save did not finish. Do not replay it. Resolve the persistence issue and reconcile the existing outcome.
- **`StepAlreadyCommitted`**: this ID already committed, so the duplicate was not run.
- **`StepIdConflict`**: an ID was reused with a different goal; use the original goal or a different ID for a genuinely new request.

A crash after `dispatching` but before a durable `executed` marker is intentionally treated as uncertain: the library cannot infer whether an external executor produced side effects. It therefore favors avoiding duplicate effects over automatic retry.
