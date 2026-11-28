# Releasing Omega Cognitive Kernel

## Project identity

- Repository: `omega-cognitive-kernel`
- Releases are prepared from a clean, up-to-date `main` unless a maintainer explicitly chooses another reviewed revision.

## Distribution channels

| Channel | Package / demo | Canonical URL |
|---------|----------------|---------------|
| crates.io (Rust) | `omega-cognitive-kernel` | https://crates.io/crates/omega-cognitive-kernel |
| PyPI (Python) | `omega-cognitive-kernel` | https://pypi.org/project/omega-cognitive-kernel/ |
| itch.io (Omega World demo) | URL not supplied yet | — |

Verify the registry pages rather than inferring published versions from Git tags.

## Preconditions

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo build --release
python3 scripts/check-release-version.py vX.Y.Z
```

The version check compares the optional tag, `Cargo.toml`, `bindings/python/Cargo.toml`, and the locked Cargo package metadata. The GitHub Release workflow runs it before building either platform.

Confirm before tagging:

- [ ] `Cargo.toml` version matches the tag (e.g. `X.Y.Z` ↔ `vX.Y.Z`)
- [ ] `bindings/python/Cargo.toml` has the same package version as `Cargo.toml`
- [ ] The source-checkout `Cargo.lock` is up to date (`cargo metadata --locked --no-deps`)
- [ ] `CHANGELOG.md` has a section for this version
- [ ] `README.md` / `docs/PRODUCT.md` version narrative is not stale
- [ ] `NOTICE` present (trademark / open-core boundary)
- [ ] Repo is **public** if this is the first open release
- [ ] CI on `main` is green

### Documentation & README sync checklist

- [ ] **Version alignment**: `README.md` version mentions, install instructions / `--tag vX.Y.Z` examples, and `Cargo.toml` / `bindings/python/Cargo.toml` match `vX.Y.Z`.
- [ ] **Commands, paths & API**: CLI subcommands (`src/main.rs`), `.omega/` persistence layout (`src/persist.rs`), configuration defaults (`src/config.rs`), and Rust API snippets in `README.md` match current code.
- [ ] **Runnable/compilable examples**: `README.md` Rust snippets and `examples/` compile cleanly (`cargo test --doc` / `cargo check --examples`).
- [ ] **CHANGELOG sync**: `CHANGELOG.md` moves `[Unreleased]` items into `## [X.Y.Z]` with the release date before tagging.
- [ ] **PyPI & Python metadata sync**: `bindings/python/README.md` (PyPI long description source) and `pyproject.toml` / `bindings/python/pyproject.toml` metadata/versions are consistent with `vX.Y.Z`.
## Cut a release (GitHub)

Preferred: trigger **Release** workflow (`workflow_dispatch`) on `main` after version bump + docs.

Manual tag path (replace `X.Y.Z` with the version already verified by `scripts/check-release-version.py`):

```bash
git checkout main
git pull --ff-only
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Pushing a `v*` tag / dispatching release runs `.github/workflows/release.yml`:

1. Tests + builds CLI (+ GUI when configured)
2. Packages platform binaries
3. Creates a GitHub Release with assets

Paste long notes from `RELEASE_NOTES_vX.Y.Z.md` into the GitHub Release body if needed.

## Install a release

After the chosen `vX.Y.Z` tag exists and its release checks are green:

```bash
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag vX.Y.Z --locked
cargo install --git https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel --tag vX.Y.Z --locked --features gui --bin omega-dashboard
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
- The repository tracks `Cargo.lock` for reproducible CLI builds; `bindings/python/Cargo.lock` pins the Python release build.
- Yank is available if a bad version ships; prefer dry-run + CI green first.
- Brand: crates.io description may mention Omega; trademark rules in `NOTICE` still apply to product naming.

## After release

- [ ] Spot-check Release page rendering (install commands, links)
- [ ] Optionally announce with a one-liner: *training-free RuleEffect patch loop, audit + rollback*
- [ ] If published to crates.io, verify docs.rs build

## PyPI (Python bindings)

`.github/workflows/release-pypi.yml` runs on `v*` tags (and `workflow_dispatch`).
It is split into three gated stages:

1. **Build** — Linux (manylinux2014 via Docker), Windows, macOS (universal2) wheels
   for CPython 3.10–3.15, plus an sdist.
2. **Test** — each built wheel is downloaded, installed, and `pytest`-tested on its
   native platform and Python version. A wheel that fails to install or pass tests
   blocks publication.
3. **Publish** — `pypa/gh-action-pypi-publish` uploads all wheels + sdist via
   **Trusted Publishing (OIDC)**, gated on the `release` environment.

### One-time Trusted Publisher setup

The publish job authenticates with PyPI via OIDC — there is **no stored API token**.
Configure it on PyPI *before* the first release:

- Project: `omega-cognitive-kernel`
- Add a **Trusted Publisher** (GitHub) with exactly:
  - Owner / repository: this repo
  - Workflow name: `release-pypi.yml`
  - Environment name: `release`
- Create a matching `release` environment in the GitHub repo settings (optionally
  require manual approval for the publish job).

If the repository, workflow filename, or environment name changes, update the PyPI
Trusted Publisher to match, otherwise the publish step will be rejected. Do **not**
retry publication by re-pushing the same tag — a version already on PyPI cannot be
overwritten; bump the version instead.

### Do not tag until green

The Python `requires-python` metadata remains `>=3.10`; the standard CI Python job,
the native-wheel workflow, and the PyPI build/test matrices cover CPython 3.10–3.15.
Confirm these matrices are green on `main` before cutting a `v*` tag. Adding newer
supported interpreters does not require raising the Python 3.10 minimum.
