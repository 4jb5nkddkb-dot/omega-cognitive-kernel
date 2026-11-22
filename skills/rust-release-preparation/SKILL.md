---
name: rust-release-preparation
description: Prepare and verify a Rust project release by aligning Cargo and language-binding versions, lockfiles, changelog, API documentation, and temporary artifacts. Use for version bumps, changelog finalization, and pre-tag release preparation without publishing.
---

# Rust Release Preparation

Coordinate the declared source version and user-facing release documentation before tagging. Keep publication, commit, and tag operations separate unless explicitly authorized.

## Workflow

1. **Establish the baseline.**
   - Read contributor instructions, the release guide, Cargo manifests, current changelog, and documentation map.
   - Inspect `git status --short --branch` and preserve existing changes. Start from the requested base revision and verify the remote-tracking branch when required.
   - Inspect the actual public API and default configuration before updating documentation. Do not infer behavior from a release request alone.

2. **Align version metadata.**
   - Update the root `[package].version` to the requested version.
   - Check language-binding manifests independently; a path dependency does not automatically align its package version. Trace dynamic package metadata such as a Python `dynamic = ["version"]` setting to its source.
   - Regenerate every tracked lockfile with Cargo, then inspect the diff for unintended dependency changes. A narrow manifest version change should not require broad dependency updates.
   - Run the repository's version-check script, when present, with the intended tag.

3. **Finalize the release narrative.**
   - Move all relevant unreleased entries under a dated release heading, retaining an empty unreleased section.
   - Read recent implementation commits so important public API changes absent from the draft changelog are included.
   - Keep correctness, safety, migration, and benchmark limitations explicit. Distinguish measured results from causal or production claims.
   - Synchronize current installation tags, dependency examples, API signatures, and safety defaults in active documentation. Preserve historical release notes and clearly label historical roadmaps.

4. **Clean only confirmed temporary artifacts.**
   - Inspect tracked patch/diagnostic files and verify their useful changes are already in source control before deleting them. Git history is sufficient archival storage unless the project specifies another archive.
   - Search for experimental forcing paths and extreme policy overrides. Do not remove legitimate benchmark configuration or normal algorithm behavior merely because it uses words such as `force` or `INFINITY`; explain the distinction.

5. **Verify what the release touches.**
   - Run the project's required checks, typically:
     ```bash
     cargo fmt --all -- --check
     cargo check --locked
     cargo test --lib --locked
     git diff --check
     ```
   - Check binding lockfile metadata using `cargo metadata --manifest-path <binding>/Cargo.toml --locked --no-deps --format-version 1` when a separate lockfile exists.
   - Compile changed Rust documentation examples where practical. Run deterministic benchmarks with their documented parameters and record actual values, not assumed outcomes.
   - For a local Python interpreter newer than the binding supports, select an installed supported interpreter with `PYO3_PYTHON`. If the release workflow uses an ABI forward-compatibility setting, test with that exact setting and disclose the distinction. Do not globally enable an untested compatibility bypass.

6. **Review and hand off.**
   - Re-read changed files, inspect the complete diff and status, and check for stale version/API/default references and whitespace errors.
   - List every modified and deleted file. Report executed verification separately from CI-only or skipped checks.
   - Provide suggested commit/tag commands and manual registry-release steps. Do not claim that tagging publishes crates.io artifacts; inspect the workflows and release guide for actual triggers and manual publication requirements.

## Guardrails

- Do not commit, tag, push, publish, or overwrite existing user work without authorization.
- Do not delete diagnostic source solely to make a search clean; remove experiment-only behavior only after tracing its runtime role.
- Do not equate a Git tag with registry availability or a successful release workflow.
- If a benchmark changes multiple variables relative to baseline, identify it as a composite comparison rather than an isolated ablation.
