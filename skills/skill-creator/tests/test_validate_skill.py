"""Regression tests for the bundled skill validator."""

import contextlib
import importlib.util
import io
import os
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts" / "validate_skill.py"
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("validate_skill", SCRIPT)
validator = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(validator)

VALID = """---
name: example-skill
description: Perform a documented task. Use when testing this workflow.
license: MIT
compatibility: Requires Python 3.10 or newer.
metadata:
  version: \"1.0\"
allowed-tools: Bash Read
---

# Example skill

Read [the reference](references/reference.md) before continuing.

```sh
# Deliberately missing links in examples are not resource links:
[example](references/not-real.md)
```
"""


class ValidatorTests(unittest.TestCase):
    def setUp(self):
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.skill = self.root / "example-skill"
        self.skill.mkdir()
        (self.skill / "references").mkdir()
        (self.skill / "references" / "reference.md").write_text("# Reference\n", encoding="utf-8")
        self.write(VALID)

    def tearDown(self):
        self.tempdir.cleanup()

    def write(self, content):
        (self.skill / "SKILL.md").write_text(content, encoding="utf-8")

    def assertProblem(self, fragment):
        problems = validator.validate_skill(self.skill)
        self.assertTrue(any(fragment in problem for problem in problems), problems)

    def test_valid_complete_skill(self):
        self.assertEqual(validator.validate_skill(self.skill), [])

    def test_name_must_match_directory_and_kebab_case(self):
        self.write(VALID.replace("example-skill", "Invalid--Name", 1))
        self.assertProblem("lowercase")
        self.assertProblem("does not match directory")

    def test_name_length_is_bounded(self):
        self.write(VALID.replace("example-skill", "a" * 65, 1))
        self.assertProblem("1-64")

    def test_missing_description_is_rejected(self):
        self.write(VALID.replace("description: Perform a documented task. Use when testing this workflow.\n", ""))
        self.assertProblem("missing required frontmatter field: description")

    def test_description_length_and_angle_brackets_are_bounded(self):
        original = "description: Perform a documented task. Use when testing this workflow."
        self.write(VALID.replace(original, "description: " + "x" * 1025))
        self.assertProblem("1-1024")
        self.write(VALID.replace(original, "description: Invalid <workflow> description."))
        self.assertProblem("angle brackets")

    def test_metadata_string_mapping_is_accepted(self):
        metadata = validator.parse_frontmatter(VALID)["metadata"]
        self.assertEqual(metadata, {"version": "1.0"})

    def test_unknown_field_is_rejected(self):
        self.write(VALID.replace("license: MIT", "version: 1\nlicense: MIT"))
        self.assertProblem("unknown frontmatter field")

    def test_compatibility_length_is_bounded(self):
        self.write(VALID.replace("Requires Python 3.10 or newer.", "x" * 501))
        self.assertProblem("1-500")

    def test_body_is_required(self):
        self.write("---\nname: example-skill\ndescription: Documented workflow.\n---\n")
        self.assertProblem("Markdown instructions")

    def test_invalid_portable_yaml_is_reported(self):
        self.write(VALID.replace("license: MIT", "metadata:\n    nested: value"))
        self.assertProblem("metadata entries")

    def test_missing_link_is_rejected_but_fence_is_ignored(self):
        self.write(VALID + "\n[broken](references/missing.md)\n")
        problems = validator.validate_skill(self.skill)
        self.assertTrue(any("missing linked resource" in problem for problem in problems))
        self.assertFalse(any("not-real" in problem for problem in problems))

    def test_external_link_is_accepted(self):
        self.write(VALID + "\n[specification](https://agentskills.io/specification)\n")
        self.assertEqual(validator.validate_skill(self.skill), [])

    def test_escaping_link_is_rejected(self):
        self.write(VALID + "\n[secret](../../../outside.md)\n")
        self.assertProblem("leaves skill directory")

    def test_private_key_is_rejected(self):
        (self.skill / "references" / "reference.md").write_text(
            "-----BEGIN PRIVATE KEY-----\nnot-a-real-key\n", encoding="utf-8"
        )
        self.assertProblem("possible credential")

    def test_backup_file_is_rejected(self):
        (self.skill / "SKILL.md.bak").write_text(VALID, encoding="utf-8")
        self.assertProblem("backup file")

    @unittest.skipUnless(hasattr(os, "symlink"), "requires symbolic links")
    def test_unsafe_symlink_is_rejected(self):
        (self.root / "outside.md").write_text("private\n", encoding="utf-8")
        (self.skill / "references" / "outside.md").symlink_to(self.root / "outside.md")
        self.assertProblem("unsafe symlink")

    def test_pack_validation_checks_each_child(self):
        pack = self.root / "skills"
        pack.mkdir()
        self.skill.rename(pack / self.skill.name)
        other = pack / "other-skill"
        other.mkdir()
        (other / "SKILL.md").write_text(
            "---\nname: other-skill\ndescription: Another workflow.\n---\n\nDo work.\n",
            encoding="utf-8",
        )
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            status = validator.main([str(pack)])
        self.assertEqual(status, 0, stderr.getvalue())
        self.assertIn("2 passed", stdout.getvalue())

    def test_cli_returns_nonzero_for_invalid_skill(self):
        self.write("not frontmatter\n")
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(validator.main([str(self.skill)]), 1)


if __name__ == "__main__":
    unittest.main()
