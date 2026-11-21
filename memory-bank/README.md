# Memory Bank

This directory is the persistent project context for Omega Cognitive Kernel. Its content is refreshed from the repository source, tests, configuration, and project documentation; it is not intended to replace the implementation or the public API docs.

## Core files

- [`projectbrief.md`](projectbrief.md): what the crate implements, the runtime objective, boundaries, and repository map.
- [`productContext.md`](productContext.md): product surfaces, defaults, auditability, recovery, and open-core boundaries.
- [`activeContext.md`](activeContext.md): current source snapshot, sensitive paths, active engineering focus, and deferred questions.
- [`systemPatterns.md`](systemPatterns.md): concrete L0/L1/L2 flow, memory model, L2 consequence loop, and persistence ordering.
- [`techContext.md`](techContext.md): Rust/features/modules, schemas, file formats, defaults, and verification entry points.
- [`progress.md`](progress.md): implemented capabilities, release/roadmap state, and what has or has not been verified.

## Maintenance rules

Update the smallest relevant file when intent, implementation state, or verification status changes. Tie claims to source paths, tests, or executed commands. Record unexecuted commands as entry points only. Do not store credentials, private data, generated runtime state, or task transcripts here.

The authoritative sources remain `src/`, tests, `Cargo.toml`, and the project documentation. When this directory disagrees with code, treat the code and its tests as authoritative and refresh the Memory Bank.
