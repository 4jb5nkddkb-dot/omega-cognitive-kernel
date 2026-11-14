# crates.io readiness (Omega Cognitive Kernel)

**Status:** packaging clean on **v0.5.6**.  
The crate is **not** on crates.io until you run a real `cargo publish` with a token.

Git install:

```toml
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.6" }
```

## Verified

- `cargo package` / `cargo publish --dry-run` — no example exclusion warnings
- `examples/**/*` in `package.include`
- GitHub tag/Release **v0.5.6** is the intended first publish version

## Pre-publish

```bash
git pull
cargo package
cargo publish --dry-run   # should be clean
cargo publish             # only when intentional + logged in
```

## After publish

- https://crates.io/crates/omega-cognitive-kernel
- docs.rs build
- optional README switch to `cargo add omega-cognitive-kernel`

## Brand

MIT covers source; trademarks stay with the holder — see [NOTICE](../NOTICE).
