#!/bin/bash
set -euo pipefail

# Build release wheels for the Python versions covered by CI.
# The distribution version is read from bindings/python/Cargo.toml by maturin.

IMAGE="quay.io/pypa/manylinux_2_28_x86_64"

for PY in 3.11 3.12; do
    echo "=== Building for Python $PY ==="
    
    # Use the interpreters available in this manylinux image; release CI covers 3.10-3.13.
    if [[ "$PY" != "3.11" && "$PY" != "3.12" ]]; then
        echo "Python $PY may not be available in manylinux_2_28, skipping..."
        continue
    fi
    
    docker run --rm -v "$(pwd):/work" -w /work/bindings/python \
        "$IMAGE" \
        bash -c "
            set -euo pipefail
            dnf install -y python$PY python$PY-devel
            alternatives --install /usr/bin/python3 python3 /usr/bin/python$PY 1
            alternatives --install /usr/bin/pip3 pip3 /usr/bin/pip$PY 1
            python3 -m pip install maturin
            export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
            python3 -m maturin build --release --locked --out /work/wheelhouse --interpreter python3
        "
    
    echo "=== Python $PY build complete ==="
done

echo "All wheels built!"
ls -la wheelhouse/