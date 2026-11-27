# Omega Cognitive Kernel v0.5.6

Packaging release: examples included in the crate tarball for crates.io.

## Highlights

- `package.include` ships `examples/**/*` (event_hook_poll, audit_http_shim, …)
- No API or state schema change vs **v0.5.5**
- Intended first **crates.io** publish candidate (`cargo publish` after dry-run)

## Install

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.6 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.6 --locked --features gui --bin omega-dashboard
```

Library:

```toml
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.6" }
```

After crates.io publish: `cargo add omega-cognitive-kernel`.

## Compatibility

Additive packaging only on 0.5.5.
