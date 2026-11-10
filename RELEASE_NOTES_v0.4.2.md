# Omega Cognitive Kernel v0.4.2

Additive **0.4.x** release: tool-call gate + operator audit panel + intervention policy config.

Does **not** break the v0.4.0 integrator freeze surface.

---

## Highlights

### Tool-call gate

```bash
cargo run --example tool_gate
```

- `ToolGateExecutor`: allow-list + deny-list, optional audit
- High-risk tool names are blocked before any side effect
- `ToolAllowed` / `ToolDenied` in `audit.jsonl`

See [docs/TOOL_GATE.md](./docs/TOOL_GATE.md).

### Audit summary

```bash
omega-kernel --state ./path/to/state.json audit --summary
```

Rollup: steps, patches, checkpoints, tool allow/deny counts, intervention block.

### Intervention policy (`omega.toml`)

```toml
[intervention]
mode = "exploration"       # or "strict"
max_level = "suggest_alternative"
friction_threshold = 2
```

See [docs/INTERVENTION_POLICY.md](./docs/INTERVENTION_POLICY.md).

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.2 --locked
```

---

## Honest scope

Still a research prototype. Tool-gate is a **safety MVP building block**, not a full LLM proxy or compliance platform.
