# Releasing Omega Kernel

## Preconditions

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Cut a release (GitHub)

```bash
git checkout main
git pull
# ensure version in Cargo.toml matches the tag (e.g. 0.3.1)
git tag -a v0.3.1 -m "v0.3.1 packaging release"
git push origin main
git push origin v0.3.1
```

Pushing `v*` runs `.github/workflows/release.yml`:

- `cargo test --release`
- builds `omega-kernel`
- uploads `omega-kernel-linux-x86_64.tar.gz`
- creates a GitHub Release

## Install a release

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.1 --locked
omega-kernel demo
```

## Optional: crates.io

Not required for git-based installs. If publishing later:

```bash
cargo publish --dry-run
cargo publish
```
