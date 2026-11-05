# Releasing Omega Cognitive Kernel

## Preconditions

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --release
```

Confirm before tagging:

- [ ] `Cargo.toml` version matches the tag (e.g. `0.3.7` ↔ `v0.3.7`)
- [ ] `CHANGELOG.md` has a section for this version
- [ ] `README.md` / `ARCHITECTURE.md` version narrative is not stale
- [ ] Repo is **public** if this is the first open release
- [ ] CI on `main` is green

## Cut a release (GitHub)

```bash
git checkout main
git pull
# optional annotated tag message — keep short; long notes go in GitHub Release body
git tag -a v0.3.7 -m "v0.3.7: L2 bias→decision mapping, strategy-decay metrics, docs"
git push origin main
git push origin v0.3.7
```

Pushing a `v*` tag runs `.github/workflows/release.yml`:

1. `cargo test --release`
2. Build `omega-kernel`
3. Package `omega-kernel-linux-x86_64.tar.gz`
4. Create a GitHub Release with the tarball attached

If you prefer a hand-written body, edit the draft release on GitHub and paste from `RELEASE_NOTES_v0.3.7.md`.

## Install a release

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.7 --locked
omega-kernel demo
omega-kernel task --episodes 4 --steps 3
```

Or download the Linux tarball from the GitHub Release assets.

## After release

- [ ] Spot-check Release page rendering (install commands, links)
- [ ] Optionally announce with a one-liner: *training-free RuleEffect patch loop, audit + rollback*
- [ ] Bump to next `-dev` work on `main` only when you start new changes (optional)
