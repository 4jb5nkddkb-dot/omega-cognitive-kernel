#!/usr/bin/env bash
set -euo pipefail

# Builds the browser and WASI library surfaces, then executes the WASI smoke
# binary in a real runtime. Set WASMTIME to an explicit binary when needed.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Prefer the rustup-managed toolchain used by the project when available.
if [ -f "${HOME}/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
fi

: "${CARGO:=cargo}"
: "${WASMTIME:=wasmtime}"

"$CARGO" build --lib --no-default-features --target wasm32-unknown-unknown
"$CARGO" build --example wasm_runtime_smoke --no-default-features --features std --target wasm32-wasip1

if ! command -v "$WASMTIME" >/dev/null 2>&1 && [ ! -x "$WASMTIME" ]; then
  echo "error: Wasmtime is required to execute the WASI smoke binary" >&2
  echo "set WASMTIME=/path/to/wasmtime and rerun" >&2
  exit 2
fi

"$WASMTIME" run --dir=/tmp \
  target/wasm32-wasip1/debug/examples/wasm_runtime_smoke.wasm
