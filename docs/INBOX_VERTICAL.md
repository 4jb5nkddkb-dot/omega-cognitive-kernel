# Inbox vertical demo

**Command**

```bash
cargo run --example inbox_vertical

# keep state / audit / actions on disk for inspection
KEEP_INBOX=1 cargo run --example inbox_vertical
```

Workspace (when kept): `./omega-inbox-demo/`

| File | Role |
|------|------|
| `omega.toml` | thresholds, max_risk, goal |
| `state.json` | cognitive snapshot |
| `audit.jsonl` | kernel audit trail |
| `actions.log` | gate decisions (executor side channel) |

---

## Story

1. An external producer pushes **text events** into an in-memory inbox (simulates webhook / queue / file drop).
2. `EventQueueObserver` implements `Observer` — each step reports queue depth, pressure, event heads.
3. `Kernel` runs the usual L0→L1→L2 loop; under load it may **commit a `RuleEffect` patch** (bounded policy, not code exec).
4. `PolicyGateExecutor` implements `Executor` — whitelist + structured log of *posture* (`hold_and_drain`, `inspect_only`, …).
5. Operators inspect **audit.jsonl** and **actions.log**.

---

## Pluggability (the product claim)

The step loop never hard-codes files or HTTP:

```rust
let observer = EventQueueObserver::new(inbox);
let executor = PolicyGateExecutor::new(whitelist, "./actions.log");
let mut kernel = Kernel::with_adapters(observer, executor);
kernel.apply_config(&KernelConfig::load("omega.toml")?);
kernel.step(&goal)?;
```

Swap observer/executor → same kernel, different world.

---

## Honest limits

- Not an NLP classifier or content-moderation product.
- "Filter / intercept" here means **policy preference under constraints** (explore / exploit / stabilize), auditable and reversible.
- Demo executor is **log-only**; production gates would call real APIs behind the same trait.

See also: [PRODUCT.md](./PRODUCT.md), [INTEGRATING.md](./INTEGRATING.md), `examples/drop_inbox_loop.rs` (directory-based variant).
