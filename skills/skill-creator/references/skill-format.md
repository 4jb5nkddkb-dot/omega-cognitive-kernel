# Agent Skills format

Use the open Agent Skills directory convention. A skill requires a `SKILL.md`; other directories are optional and should exist only when useful.

## Frontmatter

Start the file with `---`, place YAML metadata on the following lines, and close it with another `---`. The repository validator accepts the portable YAML subset shown here. Quote values containing `:` or `#` when they could be parsed as YAML syntax.

```yaml
---
name: example-skill
description: Perform a concrete outcome. Use when the user requests this specific kind of task.
license: MIT
compatibility: Requires Python 3.10 or newer for the bundled script.
metadata:
  version: "1.0"
allowed-tools: Bash Read
---
```

Only `name` and `description` are required. Optional fields are `license`, `compatibility`, `metadata`, and the experimental `allowed-tools`.

- `name`: 1–64 lowercase ASCII letters, digits, or hyphens. Do not start or end with a hyphen or use consecutive hyphens. It must exactly match the parent directory.
- `description`: 1–1024 characters. State both capability and activation conditions with specific keywords. Avoid angle brackets.
- `license`: a short license name or bundled license-file reference.
- `compatibility`: 1–500 characters describing actual product, package, platform, or network requirements. Omit it when there are no special requirements.
- `metadata`: a mapping whose keys and values are strings.
- `allowed-tools`: a space-separated string of pre-approved tools. Client support varies; do not assume it grants a tool or permission.

## Body and progressive disclosure

Put a brief purpose and the essential workflow directly in `SKILL.md`. Keep it below 500 lines; shorter is usually better. Include prerequisites, ordered actions, decision points, verification, and recovery. Move lengthy specifications, schemas, and examples to direct children of `references/` so they are loaded only when needed.

Use links relative to the skill root, such as `references/schema.md` after creating that reference file; use Markdown link syntax to make it clickable. Run scripts with an explicit interpreter when appropriate: `python3 scripts/check.py`. Do not reference resources outside the skill directory. External documentation links are allowed but are not a substitute for instructions needed offline.

Scripts should have a clear command-line contract, useful nonzero failures, and tests for nontrivial behavior. Do not add a script when a short shell command is sufficient. Assets are templates or other material used in generated output; they are not automatically executable instructions.

## Distillation checklist

A completed-task skill should capture a reusable capability, not a transcript:

- the problem and observable triggers;
- supported environment and required inputs;
- commands that were actually used, generalized only where safe;
- decisions, exceptions, and non-destructive recovery;
- verification commands and completion criteria;
- limitations and steps not exercised.

Update an existing skill when it covers the same capability. Keep separate skills for independently triggerable capabilities. Never preserve secrets, private user content, ephemeral host paths, or claims of verification that did not happen.

## Validation

From the repository root, validate one directory or every immediate child of a skill pack:

```bash
python3 skills/skill-creator/scripts/validate_skill.py skills/example-skill
python3 skills/skill-creator/scripts/validate_skill.py skills
```

The local validator uses only the Python standard library. Optionally install the official demonstration tool `skills-ref` in an isolated environment and run `skills-ref validate <skill-dir>`. The official tool provides additional reference implementation coverage; neither tool executes or proves a skill's workflow. For repeatable commands, store a runnable copy in `scripts/` or `assets/` rather than relying on a code-fence link.

Specification: <https://agentskills.io/specification>.
