#!/usr/bin/env bash
set -euo pipefail

# Validates the currently supported no_std boundary and probes the full kernel.
# The portable probe is required to pass. The full kernel check is expected to
# fail until the planned kernel-core/host-facade split is implemented.
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

echo "[no_std] probing complete kernel boundary"
set +e
"$CARGO" check --lib --no-default-features --target "$NO_STD_TARGET" >"$NO_STD_LOG" 2>&1
status=$?
set -e

if [ "$status" -eq 0 ]; then
  echo "full kernel no_std check: passed"
  exit 0
fi

echo "full kernel no_std check: not yet supported (expected boundary gap)"
echo "first blockers:"
grep -E 'error\[|error:|not found|cannot find crate' "$NO_STD_LOG" | head -20 || true

auto_accept=${ALLOW_NO_STD_GAP:-0}
if [ "$auto_accept" = "1" ]; then
  echo "ALLOW_NO_STD_GAP=1: reporting the known gap without failing"
  exit 0
fi

echo "set ALLOW_NO_STD_GAP=1 only for a non-blocking compatibility report" >&2
exit "$status"
