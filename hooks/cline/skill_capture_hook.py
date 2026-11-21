#!/usr/bin/env python3
"""Cline TaskComplete scheduler for the repository skill-creator skill.

The hook performs deterministic validation and scheduling only. Cline's model
performs the actual review on the next prompt, using pending hook context.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

PENDING = Path(".omega/skill-capture/pending.json")
INSTRUCTIONS = """<skill_capture_hook>
A project task has completed. Before handling the new user request, load and follow skills/skill-creator/SKILL.md to review the completed task in the conversation history. Use actual changes and verification evidence, not the completion text as proof. Update an existing reusable skill or create a focused one when warranted; explain when no reusable workflow exists. Do not include credentials or private data. Run python3 -B skills/skill-creator/scripts/validate_skill.py skills after any skill edit. Then run python3 -B hooks/cline/skill_capture_hook.py acknowledge to clear this pending review, including when no skill is needed. Do not commit or push. After this review, handle the user's new request.
</skill_capture_hook>"""


def emit(context: str = "", error: str = "") -> None:
    """Always allow Cline to continue; stdout is exclusively hook JSON."""
    print(json.dumps({"cancel": False, "contextModification": context, "errorMessage": error}))


def root() -> Path:
    return Path(os.environ.get("OMEGA_SKILL_CAPTURE_ROOT", Path.cwd())).resolve()


def validate(project: Path) -> str:
    script = project / "skills/skill-creator/scripts/validate_skill.py"
    if not script.is_file():
        return "skill-creator validator is missing; restore it before capturing skills"
    try:
        result = subprocess.run(
            [sys.executable, "-B", str(script), str(project / "skills")],
            cwd=project,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return f"skill validation could not run: {exc}"
    if result.returncode:
        detail = (result.stderr or result.stdout).strip()[-4000:]
        return f"skill validation failed; fix it before delivery:\\n{detail}"
    return ""


def record_completion(payload: dict, project: Path) -> str:
    metadata = payload.get("taskComplete", {}).get("taskMetadata", {})
    task_id = str(payload.get("taskId") or metadata.get("taskId") or "unknown")
    pending = project / PENDING
    pending.parent.mkdir(parents=True, exist_ok=True)
    temporary = pending.with_name(f".{pending.name}.{os.getpid()}.tmp")
    # Deliberately do not persist result, command, prompt, or conversation text.
    record = {"version": 1, "taskId": task_id, "status": "pending"}
    temporary.write_text(json.dumps(record) + "\n", encoding="utf-8")
    os.chmod(temporary, 0o600)
    temporary.replace(pending)
    return validate(project)


def deliver_pending(project: Path) -> str:
    pending = project / PENDING
    if not pending.is_file():
        return ""
    try:
        record = json.loads(pending.read_text(encoding="utf-8"))
        if record.get("version") != 1 or record.get("status") != "pending":
            raise ValueError("unsupported pending record")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return f"SKILL CAPTURE HOOK: unreadable pending state: {exc}. Inspect .omega/skill-capture/pending.json; do not delete user work."
    return INSTRUCTIONS


def acknowledge(project: Path) -> int:
    pending = project / PENDING
    try:
        pending.unlink()
    except FileNotFoundError:
        pass
    print("Skill-capture review acknowledged.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("event", nargs="?", default="complete", choices=("complete", "prompt", "acknowledge"))
    args = parser.parse_args()
    project = root()
    if args.event == "acknowledge":
        return acknowledge(project)
    try:
        payload = json.load(sys.stdin)
        if not isinstance(payload, dict):
            raise ValueError("hook input must be a JSON object")
        error = record_completion(payload, project) if args.event == "complete" else ""
        context = "" if args.event == "complete" else deliver_pending(project)
        emit(context, error)
    except Exception as exc:  # A scheduler failure must not cancel completed work.
        print(f"skill-capture hook: {exc}", file=sys.stderr)
        emit("", f"skill-capture hook failed non-blockingly: {exc}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
