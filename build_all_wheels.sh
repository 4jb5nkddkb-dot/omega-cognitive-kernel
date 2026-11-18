#!/bin/bash
set -euo pipefail

# Build wheels for Python 3.8-3.13 using manylinux_2_28 docker container
# This script runs locally and builds all 6 wheels

IMAGE="quay.io/pypa/manylinux_2_28_x86_64"
VERSION="0.5.18"

for PY in 3.8 3.9 3.10 3.11 3.12 3.13; do
    echo "=== Building for Python $PY ==="
    
    # Check if python version is available in the container
    if [[ "$PY" == "3.8" || "$PY" == "3.9" || "$PY" == "3.10" || "$PY" == "3.13" ]]; then
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
            python3 -m maturin build --release --out /work/wheelhouse --interpreter python3
        "
    
    echo "=== Python $PY build complete ==="
done

echo "All wheels built!"
ls -la wheelhouse/