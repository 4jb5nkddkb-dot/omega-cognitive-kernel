#!/usr/bin/env python3
"""Fail if a wheel's CPython and ABI tags do not match its build matrix."""

import argparse
import re
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--python-version", required=True, help="matrix version, e.g. 3.12")
    parser.add_argument("wheelhouse", type=Path, help="directory containing built .whl files")
    args = parser.parse_args()

    if not re.fullmatch(r"3\.\d+", args.python_version):
        parser.error(f"unsupported Python version format: {args.python_version}")

    expected_runtime = tuple(map(int, args.python_version.split(".")))
    if sys.version_info[:2] != expected_runtime:
        print(
            f"error: checker is running Python {sys.version_info.major}.{sys.version_info.minor}, "
            f"but matrix expects {args.python_version}",
            file=sys.stderr,
        )
        return 1

    expected_tag = f"cp{expected_runtime[0]}{expected_runtime[1]}"
    wheels = sorted(args.wheelhouse.glob("*.whl"))
    if not wheels:
        print(f"error: no wheels found in {args.wheelhouse}", file=sys.stderr)
        return 1

    mismatches = []
    for wheel in wheels:
        fields = wheel.name[:-4].split("-")
        if len(fields) < 5:
            mismatches.append(f"{wheel.name}: invalid wheel filename")
            continue

        python_tag, abi_tag = fields[-3:-1]
        if python_tag != expected_tag or abi_tag != expected_tag:
            mismatches.append(
                f"{wheel.name}: expected Python/ABI tags {expected_tag}-{expected_tag}, "
                f"found {python_tag}-{abi_tag}"
            )

    if mismatches:
        print("wheel tag mismatch:", file=sys.stderr)
        for mismatch in mismatches:
            print(f"  {mismatch}", file=sys.stderr)
        return 1

    for wheel in wheels:
        print(f"verified {wheel.name}: {expected_tag}-{expected_tag}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
