# crates.io — Omega Cognitive Kernel

**Package:** [omega-cognitive-kernel](https://crates.io/crates/omega-cognitive-kernel)

The source tree currently targets **0.6.0**, independently of registry availability. Use `cargo add omega-cognitive-kernel` for the latest published package; inspect its API and feature set rather than assuming it matches this checkout. For this checkout, use a path dependency or a verified full Git commit as described in [INTEGRATING.md](./INTEGRATING.md).

Published release installation remains available through the README only when the corresponding tag or registry version actually exists.

## Publish checklist (next versions)

1. Bump `Cargo.toml` version
2. CI green
3. `cargo publish --dry-run`
4. `cargo publish` (verified email + token)
5. Confirm crates.io + docs.rs
6. Align README / INTEGRATING version pins if needed

## Brand

MIT covers source; trademarks stay with the holder — see [NOTICE](../NOTICE).
