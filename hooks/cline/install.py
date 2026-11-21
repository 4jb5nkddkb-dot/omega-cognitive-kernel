#!/usr/bin/env python3
"""Install Cline lifecycle hooks without overwriting existing local hooks."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import shutil
import sys

NAMES = ("TaskComplete", "UserPromptSubmit")


def install(source: Path, destination: Path, update: bool = False) -> list[str]:
    messages: list[str] = []
    rules = destination.parent
    ignore_file = rules.parent / ".git/info/exclude"
    ignore_file.parent.mkdir(parents=True, exist_ok=True)
    existing = ignore_file.read_text(encoding="utf-8") if ignore_file.is_file() else ""
    if ".clinerules.local" not in existing.splitlines():
        with ignore_file.open("a", encoding="utf-8") as handle:
            handle.write("\n# Local compatibility copy created by hooks/cline/install.py\n.clinerules.local\n")
    if rules.exists() and not rules.is_dir():
        backup = rules.parent / ".omega" / "clinerules-before-hooks"
        if backup.exists():
            raise RuntimeError(f"cannot preserve {rules}: backup already exists: {backup}")
        backup.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(rules), str(backup))
        rules.mkdir()
        contents = backup.read_bytes()
        (rules / "local-rules.md").write_bytes(contents)
        # Keep a compatibility copy on disk but out of version control.
        compatibility = rules.parent / ".clinerules.local"
        compatibility.write_bytes(contents)
        messages.append(f"Preserved local rules in {rules / 'local-rules.md'}; original backup: {backup}")
    destination.mkdir(parents=True, exist_ok=True)
    for name in NAMES:
        for suffix in (("", ".ps1") if os.name == "nt" else ("",)):
            src = source / f"{name}{suffix}"
            dst = destination / f"{name}{suffix}"
            if dst.exists() or dst.is_symlink():
                if dst.is_file() and src.read_bytes() == dst.read_bytes():
                    messages.append(f"Already installed: {dst}")
                    continue
                if not update:
                    messages.append(f"Kept existing hook (not overwritten): {dst}")
                    continue
            shutil.copyfile(src, dst)
            if os.name != "nt":
                dst.chmod(0o755)
            messages.append(f"Installed: {dst}")
    return messages


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--update", action="store_true", help="replace differing installed hook files")
    parser.add_argument("--dry-run", action="store_true", help="check without changing files")
    args = parser.parse_args()
    project = args.project.resolve()
    source = project / "hooks" / "cline"
    try:
        if args.dry_run:
            for name in NAMES:
                target = project / ".clinerules" / "hooks" / name
                print(f"Would install {name} at {target} unless a different hook exists")
            return 0
        for message in install(source, project / ".clinerules" / "hooks", args.update):
            print(message)
        print("Enable Hooks in Cline Feature Settings, then start a new task or reload the window.")
        return 0
    except (OSError, RuntimeError) as exc:
        print(f"Hook installation failed: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
