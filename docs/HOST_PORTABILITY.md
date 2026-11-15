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

The kernel **core library now compiles as no_std + alloc**. The host/product facade remains std-only; this is an intentional boundary rather than a failed core build. Host surfaces include:

- `std::collections::HashMap` / `HashSet` across core modules;
- filesystem-backed persistence, checkpoints, audit logs, and directory observers;
- HTTP adapters and the current `ureq` integration;
- `std::sync::atomic` in memory bookkeeping;
- `chrono`/`uuid` feature choices and error/reporting paths that currently assume `std`;
- CLI/configuration/dashboard modules.

The current crate now uses `std` as the default feature and `--no-default-features` as the embedded core build. The core uses `alloc`, `hashbrown`, and `libm`; filesystem, audit persistence, HTTP, CLI, TOML configuration, and GUI modules are gated behind `std`. A future `kernel-core` crate can still extract this boundary, but the current conditional split already provides a compiling embedded library surface.

The current validation entry point is:

```bash
rustup target add thumbv7em-none-eabihf
./scripts/test-no-std.sh
```

The script requires both the standalone `no_std + alloc + serde` probe and the kernel core to compile. `.github/workflows/no-std.yml` and `.github/workflows/scheduled-portability.yml` run this as a blocking check and upload the compiler log as an artifact.

## Verification matrix

| Surface | Status |
|---|---|
| Native serial full tests | Passed |
| Native Clippy with `-D warnings` | Passed |
| `wasm32-wasip1` library build | Passed |
| `wasm32-unknown-unknown` library build | Passed |
| JSON snapshot hot restore at arbitrary step | Passed |
| Kernel core `no_std` build | Passed with `--no-default-features` |
| Full host facade `no_std` build | Intentionally excluded; requires filesystem/network/CLI replacement |
