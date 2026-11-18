#!/bin/bash
set -euo pipefail

export CIBW_ONLY="cp38-manylinux_x86_64 cp39-manylinux_x86_64 cp310-manylinux_x86_64 cp311-manylinux_x86_64 cp312-manylinux_x86_64 cp313-manylinux_x86_64"
export CIBW_LINUX_IMAGE="manylinux_2_28"
export CIBW_ENVIRONMENT="PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1"
export CIBW_BEFORE_BUILD="curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable && source \$HOME/.cargo/env"
export CIBW_BUILD="python -m pip install maturin && python -m maturin build --release --out {dest_dir} --interpreter python"
export CIBW_PACKAGE_DIR="bindings/python"

cibuildwheel --platform linux --output-dir wheelhouse