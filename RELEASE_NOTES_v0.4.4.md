# Omega Cognitive Kernel v0.4.4

Demo-only addition on the **0.4 freeze** line: a **localhost read-only** HTTP shim over the audit log.

---

## What's new

```bash
cargo run --example audit_http_shim
curl -s http://127.0.0.1:8787/health
curl -s http://127.0.0.1:8787/audit/summary
```

- `GET /health` · `GET /audit/summary` · `GET /audit/tail?n=20`
- No new dependencies; `127.0.0.1` only
- See [docs/AUDIT_HTTP_SHIM.md](./docs/AUDIT_HTTP_SHIM.md)

---

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.4.4 --locked
```

---

## Honest scope

This is an **integration demo**, not a public API product. No auth, no TLS, no multi-tenant.
