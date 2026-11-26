---
name: python-wheel-matrix-ci
description: Configure and verify CPython wheel build and test matrices for PyO3/maturin packages. Use when adding or changing supported Python versions, CI wheel builds, release matrices, or wheel-tag checks.
---

# Python Wheel Matrix CI

Use this workflow when a Rust-backed Python package publishes wheels for multiple CPython versions or platforms.

## Workflow

1. Read the binding metadata, supported Python classifiers and `requires-python`, plus every CI and release matrix that builds or tests wheels. Keep the declared support range and each matrix in agreement.
2. Make each build use the matrix interpreter. On a native runner, pass the active interpreter's resolved executable path to maturin; on a manylinux container, pass the matching interpreter available inside the container. Do not assume the host runner's Python selection changes the container's interpreters.
3. Before uploading a wheel, fail if the wheelhouse is empty or any filename has Python/ABI tags that differ from the matrix entry. For CPython-specific PyO3 builds, expect matching `cpXY-cpXY` tags. Run the checker under the selected matrix interpreter and verify its runtime version too.
4. Install and test each platform/version artifact on its native runner. Keep publication dependent on all wheel tests and other required release artifacts.
5. Verify workflow syntax, the checker, and its failure paths with representative matching and mismatching wheel filenames. Run the Rust feature build/tests relevant to the package, then inspect `git diff --check` and the final changed-file list.

## Recovery and limits

- If Docker or another platform is unavailable locally, distinguish the checks that ran from the matrix builds left to CI; do not claim those artifacts were verified.
- If an unrelated integration test prevents the requested feature test from compiling, preserve that failure in the handoff. Keep changes scoped and choose a narrower test target only when it still meaningfully tests the requested feature.
- Do not publish, tag, push, or open a pull request unless separately authorized.
