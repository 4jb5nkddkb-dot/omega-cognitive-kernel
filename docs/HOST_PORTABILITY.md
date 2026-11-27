# Host portability and hot snapshots
> **Language note**: Authoritative English portability matrix. For a concise Chinese summary, see [`PORTABILITY.md`](PORTABILITY.md).

## Current status

The kernel now has a host-independent library build path for WebAssembly:

```bash
cargo build --lib --no-default-features --features std --target wasm32-wasip1
cargo build --lib --no-default-features --target wasm32-unknown-unknown
```

The core browser library builds without `std`. The WASI smoke example uses the host facade and therefore builds with `--no-default-features --features std`. Running it additionally requires a Wasmtime executable; a successful compile alone is not a runtime verification.

The repeatable runtime-level integration test is:

```bash
WASMTIME=/path/to/wasmtime ./scripts/test-wasm-runtime.sh
```

It builds `wasm32-unknown-unknown`, builds the WASI smoke binary, and executes the latter in Wasmtime. The smoke binary performs four kernel steps, serializes a snapshot, restores it into a second kernel, executes one more step, and asserts the step counter and snapshot path.

The optional `getrandom/js` feature is available for WASM dependency integration; the core `new_uuid` helper itself uses a process-local atomic sequence rather than OS randomness. Network `ureq` support is not part of the no-default-features core build. These choices remove the previous native C toolchain blocker from the WASM library path.

## Snapshot guarantees

`KernelSnapshot` now includes the L2 runtime state needed for migration at an arbitrary step:

- fixed-point state, dwell count, alignment score;
- commit threshold and cooldown state;
- consequence, defensive-mode, recovery, and negative-reward counters;
- emergence detector history, thresholds, and cooldown;
- resident trust/expert/IPS state, online Dayan residual, bounded Meta-Cache, and validated pending/applied policy snapshots (schema v5);
- per-instance case-replay cursor (schema v6), restored independently of other kernels.

Restoring starts a fresh feedback cycle. It does not reapply an already executed policy to L0, and malformed or noncommitted snapshots collapse to `KEEP_L0`. Missing legacy `l2_enabled` fields default to `false`.

The public in-memory transport is:

```rust
let payload = kernel.snapshot_json()?;
receiver.apply_snapshot_json(&payload)?;
```

`KernelSnapshot::to_json` and `KernelSnapshot::from_json` provide the lower-level serde boundary. Existing snapshots remain readable: newly added fields use explicit defaults, and schema migration still rejects future versions.

The regression test `snapshot_json_roundtrips_l2_state_at_arbitrary_step` verifies that a live kernel can be serialized after several steps, restored into a new kernel, preserve L2 state and detector history, and continue for another step.

## No-std assessment

The kernel **core library now compiles as no_std + alloc**. The host/product facade remains std-only; this is an intentional boundary rather than a failed core build. Host surfaces include:

- filesystem-backed persistence, checkpoints, audit logs, and directory observers;
- HTTP adapters and network configuration;
- CLI, TOML configuration, and Dashboard modules.

The core avoids those host dependencies by using `alloc`, `hashbrown`, `libm`, and core atomics. It provides process-local UUIDs and a documented epoch fallback for time when `std` is disabled.

The current crate now uses `std` as the default feature and `--no-default-features` as the embedded core build. The core uses `alloc`, `hashbrown`, and `libm`; filesystem, audit persistence, HTTP, CLI, TOML configuration, and GUI modules are gated behind `std`. A future `kernel-core` crate can still extract this boundary, but the current conditional split already provides a compiling embedded library surface.

The current validation entry point is:

```bash
rustup target add thumbv7em-none-eabihf
./scripts/test-no-std.sh
```

The script requires both the standalone `no_std + alloc + serde` probe and the kernel core to compile. `.github/workflows/no-std.yml` and `.github/workflows/scheduled-portability.yml` run this as a blocking check and upload the compiler log as an artifact.

## Verification matrix

| Surface | Status / evidence |
|---|---|
| Native full tests | CI workflow passes; see current run artifacts |
| Native Clippy with `-D warnings` | CI workflow passes; see current run artifacts |
| `wasm32-wasip1` library build | Covered by CI |
| `wasm32-unknown-unknown` library build | Covered by CI |
| WASI runtime smoke | Covered by CI; local execution also requires Wasmtime |
| JSON snapshot hot restore at arbitrary step | `snapshot_json_roundtrips_l2_state_at_arbitrary_step` |
| Kernel core `no_std` build | `scripts/test-no-std.sh`, with `--no-default-features` |
| Full host facade `no_std` build | Intentionally excluded; requires filesystem/network/CLI replacement |
