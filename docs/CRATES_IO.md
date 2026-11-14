# crates.io readiness (Omega Cognitive Kernel)

**Status:** `cargo package` / `cargo publish --dry-run` verified for **v0.5.5**.  
The crate is **not** on crates.io until you run a real `cargo publish` with a token.

Git install remains the supported path:

```toml
omega-cognitive-kernel = { git = "https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel", tag = "v0.5.5" }
```

## Dry-run result (2026-11-14)

- Packaged successfully (~374 KiB / ~94 KiB compressed)
- Compile-from-package OK
- Upload aborted as expected under `--dry-run`
- Early dry-runs warned that `examples/*` were excluded; **`examples/**/*` is now in `package.include`** — re-run `cargo package` after pull to clear those warnings

## Why wait on a real publish

- API is additive on 0.5.x but still a research prototype
- Yanking mistakes is possible but noisy; prefer deliberate first publish
- Git tags + GitHub Releases already deliver binaries and source pins

## Pre-publish checklist

1. **Account** — crates.io account + `cargo login`
2. **Version** — `Cargo.toml` version not already on crates.io
3. **Metadata** — `license`, `description`, `repository`, `readme`, `keywords`, `categories` present
4. **Include** — `LICENSE`, `NOTICE`, `README.md`, `docs/**/*`, `examples/**/*`
5. **CI green** on the commit you will publish
6. **Dry run**

```bash
git pull
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
