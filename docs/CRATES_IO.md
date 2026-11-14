# crates.io readiness (Omega Cognitive Kernel)

**Status:** checklist only — the crate is **not** published to crates.io until you run `cargo publish` with a token.

Git install remains the supported path:

```toml
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.5" }
```

## Why wait

- API is additive on 0.5.x but still a research prototype
- Yanking mistakes is possible but noisy; prefer deliberate first publish
- Git tags + GitHub Releases already deliver binaries and source pins

## Pre-publish checklist

1. **Account** — crates.io account + `cargo login`
2. **Version** — `Cargo.toml` version not already on crates.io
3. **Metadata** — `license`, `description`, `repository`, `readme`, `keywords`, `categories` present
4. **Include** — `LICENSE`, `NOTICE`, `README.md`, `docs/**/*` in `package.include`
5. **CI green** on the commit you will publish
6. **Dry run**

```bash
cargo package
# inspect target/package/omega-cognitive-kernel-*
cargo publish --dry-run
```

7. **Real publish** (only when intentional)

```bash
cargo publish
```

## After publish

- Confirm https://crates.io/crates/omega-cognitive-kernel
- Confirm docs.rs build
- Optionally switch README install snippet to `cargo add omega-cognitive-kernel`

## Brand note

Publishing the code under MIT does not transfer trademark rights. Keep [NOTICE](../NOTICE) in the package (already in `include`).
