#!/usr/bin/env python3
"""Opt-in Cline task-completion commit/push with repository safety checks."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys

CONFIG = Path(".omega/auto-commit.json")
ALLOWED_UNTRACKED = (
    "src/",
    "tests/",
    "scripts/",
    "docs/",
    "examples/",
    "bindings/",
    "skills/",
    "hooks/",
    ".github/workflows/",
    "portable/",
)
DENIED_NAMES = {".clinerules", ".clinerules.local"}


def run(project: Path, command: list[str], check: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(
        command,
        cwd=project,
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=600,
        check=False,
    )
    if check and result.returncode:
        detail = (result.stderr or result.stdout).strip()[-4000:]
        raise RuntimeError(f"command failed ({' '.join(command)}):\n{detail}")
    return result


def enabled(project: Path) -> tuple[bool, bool]:
    config = project / CONFIG
    if not config.is_file():
        return False, False
    data = json.loads(config.read_text(encoding="utf-8"))
    return data.get("enabled") is True, data.get("push") is True


def configure(project: Path, commit: bool, push: bool) -> None:
    config = project / CONFIG
    config.parent.mkdir(parents=True, exist_ok=True)
    temporary = config.with_suffix(".tmp")
    temporary.write_text(json.dumps({"enabled": commit, "push": push}, indent=2) + "\n", encoding="utf-8")
    os.chmod(temporary, 0o600)
    temporary.replace(config)


def allowed_path(path: str) -> bool:
    candidate = PurePosixPath(path)
    if path in DENIED_NAMES or path.endswith(".bak") or candidate.is_absolute() or ".." in candidate.parts:
        return False
    if candidate.name.startswith(".") and not path.startswith(".github/workflows/"):
        return False
    if any(part in {".omega", "target", "__pycache__", ".venv"} for part in candidate.parts):
        return False
    return path.startswith(ALLOWED_UNTRACKED)


def changed_paths(project: Path) -> list[str]:
    result = run(project, ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"])
    entries = result.stdout.split("\0")
    paths = []
    for entry in entries:
        if len(entry) < 4:
            continue
        path = entry[3:]
        if " -> " in path:
            path = path.split(" -> ", 1)[1]
        paths.append(path)
    return [path for path in paths if path != str(CONFIG)]


def untracked_paths(project: Path) -> list[str]:
    return [
        path
        for path in run(project, ["git", "ls-files", "--others", "--exclude-standard", "-z"]).stdout.split("\0")
        if path and path != str(CONFIG)
    ]


def reject_unreviewed_untracked(project: Path) -> None:
    rejected = [path for path in untracked_paths(project) if not allowed_path(path)]
    if rejected:
        raise RuntimeError("auto-commit: untracked paths require review: " + ", ".join(rejected))


def commit_and_push(project: Path) -> str:
    paths = changed_paths(project)
    if not paths or paths == [str(CONFIG)]:
        return "auto-commit: no changes"
    reject_unreviewed_untracked(project)

    run(project, ["cargo", "fmt", "--all"])
    reject_unreviewed_untracked(project)
    run(project, ["cargo", "fmt", "--all", "--", "--check"])
    run(project, ["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"])
    run(project, ["cargo", "test", "--locked"])
    run(project, [sys.executable, "-B", "skills/skill-creator/scripts/validate_skill.py", "skills"])
    run(project, ["git", "diff", "--check"])

    run(project, ["git", "add", "-u", "--", "."])
    untracked = untracked_paths(project)
    if untracked:
        run(project, ["git", "add", "--"] + untracked)
    if run(project, ["git", "diff", "--cached", "--quiet"], check=False).returncode == 0:
        return "auto-commit: no changes after formatting"
    subject = run(project, ["git", "log", "-1", "--format=%s"]).stdout.strip()
    message = f"chore: verified task changes after {subject}" if subject else "chore: verified task changes"
    run(project, ["git", "commit", "-m", message])
    pushed = False
    if enabled(project)[1]:
        branch = run(project, ["git", "branch", "--show-current"]).stdout.strip()
        if not branch:
            raise RuntimeError("auto-commit: committed locally; cannot push a detached HEAD")
        run(project, ["git", "push", "origin", f"HEAD:{branch}"])
        pushed = True
    return f"auto-commit: committed verified changes; pushed={str(pushed).lower()}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--enable", action="store_true")
    parser.add_argument("--disable", action="store_true")
    args = parser.parse_args()
    project = args.project.resolve()
    try:
        if args.enable or args.disable:
            configure(project, args.enable, args.enable)
            print(f"auto-commit enabled={args.enable}; push={args.enable}")
            return 0
        if not enabled(project)[0]:
            return 0
        print(commit_and_push(project), file=sys.stderr)
    except (OSError, RuntimeError, ValueError, subprocess.TimeoutExpired) as exc:
        # A lifecycle hook must not cancel the completed Cline task.
        print(f"auto-commit: skipped: {exc}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
