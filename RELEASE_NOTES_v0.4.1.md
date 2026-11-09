# Omega Cognitive Kernel v0.4.1

Patch release on the **0.4 freeze line**: product demo + docs only.

**No breaking changes** to the integrator API frozen in v0.4.0.

---

## What's new

### Inbox vertical demo

```bash
cargo run --example inbox_vertical
KEEP_INBOX=1 cargo run --example inbox_vertical
```

- Scripted text/event stream into an in-memory inbox
- Custom `Observer` (queue depth / pressure) + `Executor` (policy gate, log-only)
- Shows optional `RuleEffect` under load and prints **audit** + **actions.log**
- Guide: [docs/INBOX_VERTICAL.md](./docs/INBOX_VERTICAL.md)

### README

- Inbox vertical promoted as the recommended product demo
- Links to PRODUCT / INTEGRATING / INBOX_VERTICAL / API_FREEZE docs

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.1 --locked
```

Still **git-tag preferred** (crates.io optional, not required).

---

## Compatibility

- Same freeze list as [docs/API_FREEZE_0.4.md](./docs/API_FREEZE_0.4.md)
- Examples are illustrative and may change without a major bump

See [CHANGELOG.md](./CHANGELOG.md).
