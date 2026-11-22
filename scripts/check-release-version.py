#!/usr/bin/env python3
"""Check release tag and package versions without third-party dependencies."""

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def cargo_lock_version() -> str:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    packages = json.loads(result.stdout)["packages"]
    return next(p["version"] for p in packages if p["name"] == "omega-cognitive-kernel")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag", nargs="?", help="release tag, for example v0.6.0")
    args = parser.parse_args()
    with (ROOT / "Cargo.toml").open("rb") as handle:
        rust_version = tomllib.load(handle)["package"]["version"]
    with (ROOT / "bindings/python/Cargo.toml").open("rb") as handle:
        python_version = tomllib.load(handle)["package"]["version"]

    expected = {"Rust package": rust_version, "Python package": python_version}
    lock_version = cargo_lock_version()
    expected["Cargo.lock (via cargo metadata)"] = lock_version
    if args.tag:
        match = re.fullmatch(r"v(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)", args.tag)
        if not match:
            print(f"error: unsupported release tag: {args.tag}", file=sys.stderr)
            return 2
        expected["release tag"] = match.group(1)

    mismatches = [f"{name}: {version}" for name, version in expected.items() if version != rust_version]
    if mismatches:
        print("release version mismatch:", file=sys.stderr)
        for mismatch in mismatches:
            print(f"  {mismatch}", file=sys.stderr)
        print(f"  expected: {rust_version}", file=sys.stderr)
        return 1
    print(f"release versions consistent: {rust_version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
