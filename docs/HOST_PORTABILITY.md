# Host portability and hot snapshots

## Current status

The kernel now has a host-independent library build path for WebAssembly:

```bash
cargo build --lib --no-default-features --target wasm32-wasip1
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

Both targets compile successfully in CI-like checks. The binary CLI and filesystem/network adapters remain host surfaces; the wasm library build keeps the core orchestration, memory, L1/L2 state machine, serde types, and snapshot logic while making HTTP adapters return an explicit `ExecutionBlocked` result instead of pulling native networking dependencies into wasm.

The repeatable runtime-level integration test is:

```bash
WASMTIME=/path/to/wasmtime ./scripts/test-wasm-runtime.sh
```

It builds `wasm32-unknown-unknown`, builds the WASI smoke binary, and executes the latter in Wasmtime. The smoke binary performs four kernel steps, serializes a snapshot, restores it into a second kernel, executes one more step, and asserts the step counter and snapshot path.

The `uuid/js` feature supplies randomness for `wasm32-unknown-unknown`; `ureq` is now a non-wasm target dependency. This removes the previous `ring`/native C toolchain blocker from the wasm library path.

## Snapshot guarantees

`KernelSnapshot` now includes the L2 runtime state needed for migration at an arbitrary step:

- fixed-point state, dwell count, alignment score;
- commit threshold and cooldown state;
- consequence, defensive-mode, recovery, and negative-reward counters;
- emergence detector history, thresholds, and cooldown.

The public in-memory transport is:

```rust
let payload = kernel.snapshot_json()?;
receiver.apply_snapshot_json(&payload)?;
```

`KernelSnapshot::to_json` and `KernelSnapshot::from_json` provide the lower-level serde boundary. Existing snapshots remain readable: newly added fields use explicit defaults, and schema migration still rejects future versions.

The regression test `snapshot_json_roundtrips_l2_state_at_arbitrary_step` verifies that a live kernel can be serialized after several steps, restored into a new kernel, preserve L2 state and detector history, and continue for another step.

## No-std assessment

The full product kernel is **not yet no_std**. The remaining blockers are deliberate host/product surfaces and standard-library assumptions:

- `std::collections::HashMap` / `HashSet` across core modules;
- filesystem-backed persistence, checkpoints, audit logs, and directory observers;
- HTTP adapters and the current `ureq` integration;
- `std::sync::atomic` in memory bookkeeping;
- `chrono`/`uuid` feature choices and error/reporting paths that currently assume `std`;
- CLI/configuration/dashboard modules.

The recommended next split is a `kernel-core` no-std-plus-alloc crate containing types, L0 decision/predictor, memory, L1/L2 state machines, and an allocation-free adapter trait boundary. The existing crate should remain the host facade implementing persistence, audit, networking, filesystem observers, and CLI. That split is safer than applying `no_std` conditionals throughout the current monolith and preserves the existing native API.

The current validation entry point is:

```bash
rustup target add thumbv7em-none-eabihf
./scripts/test-no-std.sh
```

The script requires the standalone `no_std + alloc + serde` probe to compile, then probes the full kernel and prints its blockers. `ALLOW_NO_STD_GAP=1` converts the known full-kernel gap into a non-blocking report. `.github/workflows/no-std.yml` runs the probe as a required job and keeps a separate full-kernel boundary report with the compiler log as an artifact.

## Verification matrix

| Surface | Status |
|---|---|
| Native serial full tests | Passed |
| Native Clippy with `-D warnings` | Passed |
| `wasm32-wasip1` library build | Passed |
| `wasm32-unknown-unknown` library build | Passed |
| JSON snapshot hot restore at arbitrary step | Passed |
| Full product `no_std` build | Not yet supported; requires core/facade split |
