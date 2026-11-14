# Releasing Omega Cognitive Kernel

## Preconditions

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --release
```

Confirm before tagging:

- [ ] `Cargo.toml` version matches the tag (e.g. `0.5.4` ↔ `v0.5.4`)
- [ ] `CHANGELOG.md` has a section for this version
- [ ] `README.md` / `docs/PRODUCT.md` version narrative is not stale
- [ ] `NOTICE` present (trademark / open-core boundary)
- [ ] Repo is **public** if this is the first open release
- [ ] CI on `main` is green

## Cut a release (GitHub)

Preferred: trigger **Release** workflow (`workflow_dispatch`) on `main` after version bump + docs.

Manual tag path:

```bash
git checkout main
git pull
git tag -a v0.5.4 -m "v0.5.4: CLI strategy-weight/audit-poll, dashboard audit follow"
git push origin main
git push origin v0.5.4
```

Pushing a `v*` tag / dispatching release runs `.github/workflows/release.yml`:

1. Tests + builds CLI (+ GUI when configured)
2. Packages platform binaries
3. Creates a GitHub Release with assets

Paste long notes from `RELEASE_NOTES_vX.Y.Z.md` into the GitHub Release body if needed.

## Install a release

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.4 --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.5.4 --locked --features gui --bin omega-dashboard
omega-kernel demo
omega-kernel status
```

Or download binaries from the GitHub Release assets.

## crates.io (optional, when ready)

Do **not** publish on every tag. Publish when the public API has been stable for a stretch and you want `cargo add omega-cognitive-kernel`.

Checklist:

- [ ] Logged in: `cargo login` (crates.io token)
- [ ] Metadata OK: `license`, `description`, `repository`, `readme`, `keywords`, `categories`
- [ ] `include` lists `LICENSE`, `NOTICE`, `README.md`, `docs/**/*`
- [ ] Dry run: `cargo package --allow-dirty` then inspect `target/package/`
- [ ] `cargo publish --dry-run`
- [ ] Real: `cargo publish` (version must be new on crates.io; cannot overwrite)

Notes:

- This crate ships binaries (`omega-kernel`, `omega-audit-panel`; GUI behind `gui` feature). That is fine for crates.io.
- `Cargo.lock` is gitignored (library-style). Dependents resolve their own lock.
- Yank is available if a bad version ships; prefer dry-run + CI green first.
- Brand: crates.io description may mention Omega; trademark rules in `NOTICE` still apply to product naming.

## After release

- [ ] Spot-check Release page rendering (install commands, links)
- [ ] Optionally announce with a one-liner: *training-free RuleEffect patch loop, audit + rollback*
- [ ] If published to crates.io, verify docs.rs build
