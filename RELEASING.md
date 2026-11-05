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
git tag -a v0.3.2 -m "v0.3.2 RuleEffect expansion + strategy-aware patches"
git push origin main
git push origin v0.3.2
```

Pushing `v*` runs `.github/workflows/release.yml`.

## Install a release

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag v0.3.2 --locked
omega-kernel demo
```
