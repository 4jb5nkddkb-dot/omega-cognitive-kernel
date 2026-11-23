"""End-to-end tests for Cline's executable hook protocol."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "hooks" / "cline" / "skill_capture_hook.py"
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("skill_capture_hook", SCRIPT)
hook = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(hook)
INSTALL_SPEC = importlib.util.spec_from_file_location("install_hooks", ROOT / "hooks/cline/install.py")
installer = importlib.util.module_from_spec(INSTALL_SPEC)
INSTALL_SPEC.loader.exec_module(installer)
AUTO_SPEC = importlib.util.spec_from_file_location("auto_commit", ROOT / "hooks/cline/auto_commit.py")
auto_commit = importlib.util.module_from_spec(AUTO_SPEC)
AUTO_SPEC.loader.exec_module(auto_commit)


class AutoCommitTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.project = Path(self.tempdir.name)
        self.env = os.environ.copy()
        self.env["OMEGA_SKILL_CAPTURE_ROOT"] = str(self.project)

    def tearDown(self):
        self.tempdir.cleanup()

    def test_disabled_by_default_and_enable_roundtrip(self):
        self.assertEqual(auto_commit.enabled(self.project), (False, False))
        auto_commit.configure(self.project, True, True)
        self.assertEqual(auto_commit.enabled(self.project), (True, True))
        auto_commit.configure(self.project, False, False)
        self.assertEqual(auto_commit.enabled(self.project), (False, False))

    def test_untracked_path_policy(self):
        self.assertTrue(auto_commit.allowed_path("src/new.rs"))
        self.assertTrue(auto_commit.allowed_path(".github/workflows/new.yml"))
        self.assertFalse(auto_commit.allowed_path(".clinerules.local"))
        self.assertFalse(auto_commit.allowed_path("secret.env"))
        self.assertFalse(auto_commit.allowed_path("src/../../outside"))

    def test_git_fixture_commits_and_pushes_only_after_verification(self):
        remote = self.project / "remote.git"
        work = self.project / "work"
        subprocess.run(["git", "init", "--bare", str(remote)], check=True, capture_output=True)
        subprocess.run(["git", "init", str(work)], check=True, capture_output=True)
        auto_commit.run(work, ["git", "config", "user.email", "test@example.invalid"])
        auto_commit.run(work, ["git", "config", "user.name", "Test"])
        (work / "tracked.txt").write_text("before\n", encoding="utf-8")
        auto_commit.run(work, ["git", "add", "tracked.txt"])
        auto_commit.run(work, ["git", "commit", "-m", "initial"])
        auto_commit.run(work, ["git", "branch", "-M", "main"])
        auto_commit.run(work, ["git", "remote", "add", "origin", str(remote)])
        auto_commit.run(work, ["git", "config", "info.exclude", ".omega/"])
        auto_commit.configure(work, True, True)
        (work / "tracked.txt").write_text("after\n", encoding="utf-8")

        original_run = auto_commit.run

        def fake_verify(project, command, check=True):
            if command[0] in {"cargo", sys.executable}:
                return subprocess.CompletedProcess(command, 0, "", "")
            return original_run(project, command, check)

        auto_commit.run = fake_verify
        try:
            self.assertIn("pushed=true", auto_commit.commit_and_push(work))
        finally:
            auto_commit.run = original_run
        self.assertEqual(auto_commit.changed_paths(work), [])
        self.assertIn("after", auto_commit.run(work, ["git", "show", "HEAD:tracked.txt"]).stdout)
        self.assertEqual(
            auto_commit.run(work, ["git", "log", "-1", "--format=%s"]).stdout.strip(),
            "chore: verify completed task changes",
        )
        self.assertEqual(
            auto_commit.run(work, ["git", "rev-parse", "HEAD"]).stdout,
            auto_commit.run(work, ["git", "rev-parse", "origin/main"]).stdout,
        )

    def test_verification_failure_does_not_commit(self):
        subprocess.run(["git", "init", str(self.project)], check=True, capture_output=True)
        auto_commit.run(self.project, ["git", "config", "info.exclude", ".omega/"])
        auto_commit.configure(self.project, True, True)
        (self.project / "src").mkdir()
        (self.project / "src/new.rs").write_text("fn main() {}\n", encoding="utf-8")
        original_run = auto_commit.run

        def fail_test(project, command, check=True):
            if command[0] == "cargo" and command[:2] != ["cargo", "test"]:
                return subprocess.CompletedProcess(command, 0, "", "")
            if command[:2] == ["cargo", "test"]:
                raise RuntimeError("simulated test failure")
            return original_run(project, command, check)

        auto_commit.run = fail_test
        try:
            with self.assertRaisesRegex(RuntimeError, "simulated test failure"):
                auto_commit.commit_and_push(self.project)
        finally:
            auto_commit.run = original_run
        self.assertNotEqual(
            auto_commit.run(self.project, ["git", "status", "--porcelain"]).stdout,
            "",
        )


class HookTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.project = Path(self.tempdir.name)
        self.env = os.environ.copy()
        self.env["OMEGA_SKILL_CAPTURE_ROOT"] = str(self.project)

    def tearDown(self):
        self.tempdir.cleanup()

    def invoke(self, event, payload=None):
        if payload is None:
            payload = {"taskId": "task-1", "taskComplete": {"taskMetadata": {"result": "secret result"}}}
        return subprocess.run(
            [sys.executable, "-B", str(SCRIPT), event],
            input=json.dumps(payload), text=True, capture_output=True,
            env=self.env, check=False, timeout=5,
        )

    def test_complete_persists_no_completion_text_and_prompt_delivers_instruction(self):
        completed = self.invoke("complete")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertFalse(json.loads(completed.stdout)["cancel"])
        pending = self.project / hook.PENDING
        self.assertTrue(pending.is_file())
        self.assertNotIn("secret result", pending.read_text(encoding="utf-8"))
        prompted = self.invoke("prompt", {"taskId": "task-2", "userPromptSubmit": {"prompt": "next"}})
        context = json.loads(prompted.stdout)["contextModification"]
        self.assertIn("skill-creator/SKILL.md", context)
        self.assertIn("acknowledge", context)
        self.assertNotIn("secret result", context)

    def test_acknowledge_stops_future_injection_and_is_idempotent(self):
        self.invoke("complete")
        env = self.env.copy()
        first = subprocess.run([sys.executable, "-B", str(SCRIPT), "acknowledge"], env=env, capture_output=True, check=False)
        second = subprocess.run([sys.executable, "-B", str(SCRIPT), "acknowledge"], env=env, capture_output=True, check=False)
        self.assertEqual(first.returncode, 0)
        self.assertEqual(second.returncode, 0)
        self.assertEqual(json.loads(self.invoke("prompt", {}).stdout)["contextModification"], "")

    def test_missing_validator_is_nonblocking(self):
        result = self.invoke("complete")
        self.assertEqual(result.returncode, 0)
        self.assertIn("validator is missing", json.loads(result.stdout)["errorMessage"])

    def test_invalid_json_does_not_cancel_task(self):
        result = subprocess.run([sys.executable, "-B", str(SCRIPT)], input="not json", text=True, capture_output=True, env=self.env, check=False)
        self.assertEqual(result.returncode, 0)
        self.assertFalse(json.loads(result.stdout)["cancel"])

    def test_installer_preserves_flat_rules_and_existing_hooks(self):
        rules = self.project / ".clinerules"
        rules.write_text("local instructions\n", encoding="utf-8")
        source = self.project / "hooks/cline"
        source.mkdir(parents=True)
        for name in ("TaskComplete", "UserPromptSubmit"):
            (source / name).write_text("#!/usr/bin/env python3\n", encoding="utf-8")
        messages = installer.install(source, rules / "hooks")
        self.assertTrue(any("Preserved local rules" in message for message in messages))
        self.assertEqual((rules / "local-rules.md").read_text(encoding="utf-8"), "local instructions\n")
        original = self.project / ".omega/clinerules-before-hooks"
        self.assertEqual(original.read_text(encoding="utf-8"), "local instructions\n")
        hook_path = rules / "hooks/TaskComplete"
        hook_path.write_text("custom hook\n", encoding="utf-8")
        installer.install(source, rules / "hooks")
        self.assertEqual(hook_path.read_text(encoding="utf-8"), "custom hook\n")


if __name__ == "__main__":
    unittest.main()
