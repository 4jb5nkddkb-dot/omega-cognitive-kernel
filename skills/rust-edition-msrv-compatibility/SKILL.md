---
name: rust-edition-msrv-compatibility
description: Diagnose and fix Rust Edition, MSRV, Cargo.lock, and Clippy compatibility failures across Rust toolchains. Use when cargo clippy/test fails because a dependency requires a newer rustc/Cargo or an Edition 2024 manifest cannot be parsed.
---

# Rust Edition/MSRV Compatibility

Use this workflow for Rust projects that declare an MSRV, support multiple Rust versions, or fail during dependency resolution before compilation.

## Workflow

1. **Capture the contract and reproduce the failure.**
   - Read `rust-version`, Cargo features, CI commands, and repository instructions.
   - Record `rustc --version` and `cargo --version`.
   - Run the exact failing command, normally:
     ```bash
     cargo clippy --all-targets -- -D warnings
     ```
   - Preserve the first dependency/Edition/MSRV error; do not treat later compiler errors as independent until the dependency issue is fixed.

2. **Classify the failure.**
   - `feature edition2024 is required`: the active Cargo is too old to parse the selected crate manifest. Pin or downgrade that dependency and its parent dependency chain.
   - `package ... requires rustc X or newer`: the lockfile contains a release above the declared MSRV. Select the newest compatible release.
   - Feature-only failure: determine whether the dependency is from an optional GUI/platform feature. Keep the MSRV job on the portable/default feature set and validate optional features with a newer stable toolchain.
   - Target/toolchain failure: distinguish a missing target from a source or dependency failure.

3. **Inspect the dependency chain.**
   - Use `Cargo.lock`, `cargo tree -i <crate>`, and the crates.io index/cache when available.
   - Identify the direct dependency that permits the incompatible transitive version.
   - Prefer a direct exact version pin for important runtime/build dependencies, then use `cargo update -p <crate> --precise <version>` to update the lockfile.
   - Typical compatibility pins must be evidence-based; do not blindly downgrade unrelated crates.

4. **Make the dependency graph reproducible.**
   - Commit `Cargo.lock` for binaries, CLI tools, applications, and integration repositories. Remove an overly broad `Cargo.lock` ignore rule when necessary.
   - Add `--locked` to CI `cargo check`, `clippy`, `test`, and release build commands.
   - Do not run a broad `cargo update` after applying pins unless you are prepared to reapply every compatibility constraint; it can restore Edition 2024 packages.
   - Avoid unnecessary features that introduce random, network, GUI, or platform dependencies. Replace them with existing portable helpers where appropriate.

5. **Separate MSRV from optional platform validation.**
   - The MSRV job should use the declared minimum toolchain and portable/default features:
     ```bash
     cargo fmt --all -- --check
     cargo clippy --locked --all-targets -- -D warnings
     cargo test --locked --all-targets
     ```
   - Validate optional GUI/platform features in a stable-toolchain job. Do not claim that an old MSRV covers platform backends that require newer Rust.
   - Add an explicit CI job using `dtolnay/rust-toolchain@<msrv>` when the project has a meaningful MSRV.

6. **Verify progressively.**
   Run, in order:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo test --locked --all-targets -- --test-threads=1
   ```
   Then run project-specific no_std, WASM, release, and optional-feature checks. If a target is not installed, report that separately rather than changing source code to hide it.

7. **Review and deliver.**
   - Run `git diff --check` and inspect the lockfile diff for unintended major changes.
   - Confirm the declared `rust-version`, lockfile, CI toolchain, and CI commands agree.
   - Summarize which crates were pinned and why, which checks passed, and which checks were unavailable due to the local toolchain.
   - Commit only when authorized; use the repository’s attribution policy for authorized commits.

## Practical guardrails

- Never solve an Edition 2024 parsing error by changing the project’s Edition unless the project intentionally migrates.
- Do not silently raise the MSRV just to make one machine pass; first check whether a compatible dependency release exists.
- Do not edit `Cargo.lock` manually when `cargo update -p ... --precise ...` can produce a valid lockfile.
- Treat optional GUI and OS backends as a separate compatibility surface.
- If a broad `cargo update` reintroduces incompatible packages, restore the previous lockfile and reapply targeted precise updates.

## Output checklist

Report:

- failing toolchain and exact root dependency error;
- direct and transitive dependencies changed;
- whether `Cargo.lock` is now tracked and commands use `--locked`;
- MSRV and stable/optional-feature CI coverage;
- verification results and any unavailable target/toolchain limitation;
- commit and push status, if performed.
