# Cline skill-capture hooks

This integration uses Cline's executable workspace-hook protocol. It requires Python 3.10+ and Cline hooks enabled in **Feature Settings**. It does not need API keys, network access, Node.js, or a package installation.

## What runs automatically

1. Cline invokes `.clinerules/hooks/TaskComplete` whenever it marks a task complete. The hook writes a minimal pending record under the Git-ignored `.omega/skill-capture/` directory and validates `skills/`. It does not store prompts, commands, completion results, or conversation history.
2. Cline invokes `.clinerules/hooks/UserPromptSubmit` before the next model request. If a review is pending, the hook injects an instruction to load `skills/skill-creator/SKILL.md`, review the completed work, and validate any skill changes.
3. The agent performs the actual knowledge distillation. A command script cannot infer a reusable skill from a task without model context. After the review (including a justified no-op), the agent runs the `acknowledge` command to remove the pending record.

Cline does not expose an additional unattended model turn through this command-hook API. Therefore the automatic trigger and queue are immediate; model-authored skill creation occurs before the next user request is handled. A persistent project rule also instructs agents to perform this review during final verification, avoiding the delay in ordinary use.

## Install

From the repository root:

```bash
python3 -B hooks/cline/install.py
```

The installer is idempotent and never overwrites a differing existing hook by default. If `.clinerules` is a local rules file, it moves the original to `.omega/clinerules-before-hooks`, creates `.clinerules/local-rules.md`, and writes a `.clinerules.local` compatibility copy for clients that only read a single file. The compatibility file is excluded via `.git/info/exclude`, not committed. Review those local changes before using another Git branch or client. The `.clinerules` ignore rule remains unchanged; only `hooks/cline/` is version-controlled.

To update an existing copy of these two hooks explicitly:

```bash
python3 -B hooks/cline/install.py --update
```

Reload the VS Code window after installation if hook discovery does not refresh. Check **Settings → Feature Settings → Enable Hooks** and ensure the Unix files remain executable. Windows uses the corresponding `.ps1` entry points.

## Optional verified task commit and push

Task completion can also run repository verification, commit the changes, and push the current branch. This is deliberately opt-in and separate from skill capture:

```bash
python3 -B hooks/cline/install.py --update
python3 -B hooks/cline/auto_commit.py --enable
# Disable both operations:
python3 -B hooks/cline/auto_commit.py --disable
```

The configuration is local-only `.omega/auto-commit.json`. Before committing, the automation rejects unreviewed untracked paths outside project source/test/docs/script/hook/workflow directories, then runs `cargo fmt`, Clippy, `cargo test --locked`, and skill validation. Any failure skips the commit and push. The hook never cancels the completed Cline task; inspect stderr for a skipped run. It uses a generic message derived from the previous commit, so use a manual commit for selective staging or a curated commit message.

## Test

```bash
python3 -B -m unittest discover -s hooks/cline/tests -v
python3 -B skills/skill-creator/scripts/validate_skill.py skills
```

The tests exercise the actual stdin/stdout JSON protocol, nonblocking errors, redacted pending state, acknowledgement, and preservation of local rules. They do not simulate a model response or claim to verify a live VS Code UI task.

## Acknowledge or inspect manually

```bash
python3 -B hooks/cline/skill_capture_hook.py acknowledge
cat .omega/skill-capture/pending.json
```

Only acknowledge after the review is done. To disable the integration, turn off these files in Cline's Hooks panel or turn off Execute permissions. Do not remove unrelated hooks or local rules.

Reference: [Cline hook protocol](https://github.com/cline/cline/blob/main/.clinerules/hooks/README.md).
