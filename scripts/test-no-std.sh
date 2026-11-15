#!/usr/bin/env bash
set -euo pipefail

# Validates the no_std core boundary and the standalone embedded probe. Host
# persistence, filesystem, networking, CLI, and GUI surfaces are excluded by
# --no-default-features and remain available through the std facade.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [ -f "${HOME}/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "${HOME}/.cargo/env"
fi

: "${CARGO:=cargo}"
: "${NO_STD_TARGET:=thumbv7em-none-eabihf}"
: "${NO_STD_LOG:=${TMPDIR:-/tmp}/omega-no-std-kernel.log}"

if ! rustup target list --installed | grep -qx "$NO_STD_TARGET"; then
  echo "error: Rust target $NO_STD_TARGET is not installed" >&2
  echo "install with: rustup target add $NO_STD_TARGET" >&2
  exit 2
fi

echo "[no_std] compiling serde/alloc embedded probe"
"$CARGO" check \
  --manifest-path portable/no_std_probe/Cargo.toml \
  --target "$NO_STD_TARGET"

echo "[no_std] compiling kernel core boundary"
"$CARGO" check --lib --no-default-features --target "$NO_STD_TARGET" \
  >"$NO_STD_LOG" 2>&1
echo "kernel core no_std check: passed"
