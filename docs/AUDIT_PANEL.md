# Audit panel (operator view)

Lightweight rollup over `audit.jsonl` — counts, action histogram, last step metrics.

This is the **terminal / library** panel (commercial web dashboard can reuse the same `AuditSummary`).

---

## CLI

```bash
# after any run that wrote state + audit beside it
omega-kernel --state ./path/to/state.json audit --summary

# raw tail (existing)
omega-kernel --state ./path/to/state.json audit --tail 30
```

## Example

```bash
cargo run --example audit_panel
# or
cargo run --example audit_panel -- ./omega-inbox-demo/state.json
```

## Library

```rust
use omega_cognitive_kernel::audit::AuditLog;

let log = AuditLog::beside_state(std::path::Path::new("./state.json"));
let summary = log.summarize()?;
println!("patches_accepted={}", summary.patches_accepted);
```

Public types: `AuditSummary`, `AuditLog::read_all`, `AuditLog::summarize`.

---

## Pair with inbox vertical

```bash
KEEP_INBOX=1 cargo run --example inbox_vertical
omega-kernel --state ./omega-inbox-demo/state.json audit --summary
```

See [INBOX_VERTICAL.md](./INBOX_VERTICAL.md).
