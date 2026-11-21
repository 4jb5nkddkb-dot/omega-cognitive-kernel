---
name: skill-creator
description: Create, review, validate, and evolve compatible skills inside the repository. Use when adding a new skill directory, updating an existing SKILL.md, designing bundled references or scripts, or checking a skill before commit. Also use after successfully completing a task to distill its reusable workflow into the repository skill pack.
---

# Skill Creator

Turn verified work into reusable, inspectable skills. A successful task is an opportunity to review its workflow, not a requirement to save every action. Preserve repository instructions and existing user changes throughout.

## Repository contract

- Store repository skills in `skills/<kebab-case-name>/SKILL.md` unless repository instructions specify another location.
- Read `.clinerules`, contributor guidance, and neighboring skills before editing. Check `git status` and never overwrite unrelated work.
- Keep the supplied skill name and description when creating a requested skill. For updates, preserve the existing directory and `name` unless renaming is explicitly requested.
- Never record credentials, private data, one-off business decisions, or unverified claims. Do not commit `.bak` files, local tool configuration, or generated runtime state.
- Run `python3 skills/skill-creator/scripts/validate_skill.py <skill-dir>` before delivery. Validate the complete pack when changing shared conventions.

## Capture a completed task

1. **Confirm completion.** Check the requested outcome and its actual verification results. Distinguish executed commands from proposed commands. If the outcome is unverified, say so and do not present the workflow as proven.
2. **Assess reuse.** Extract the repeatable problem, triggers, inputs, outputs, prerequisites, decision points, failure recovery, and completion checks. Skip a one-off operation with no transferable procedure; explain why it was not captured.
3. **Find a home.** Inspect existing skill names, descriptions, and bodies. Update a skill that already owns the workflow rather than adding a near duplicate. Create a focused skill only for a distinct reusable capability.
4. **Generalize evidence.** Replace task-specific identifiers, secrets, local absolute paths, and temporary state with documented parameters or repository-relative examples. Preserve useful commands exactly when they remain applicable. Never invent a successful run.
5. **Write the smallest complete skill.** State what the skill does and when to use it in frontmatter. Put the essential ordered workflow in `SKILL.md`; move detailed specifications to `references/` and reusable automation to `scripts/`. Use `assets/` only for resources needed in output.
6. **Review against the source work.** Confirm each prerequisite and step is supported by the task, alternatives are not presented as mandatory, and recovery steps do not destroy user work. Record known environment limitations.
7. **Validate and deliver.** Run bundled scripts or focused tests where practical, then run the validator and `git diff --check`. Report created or updated skills, checks performed, unverified steps, and any task intentionally not captured.

For tasks whose explicit purpose is to author a skill, capture and validate that skill directly; do not recursively create another skill describing the act of creating it. When a broader successful task warrants capture, perform this review during the task's final verification, before reporting completion. The project may install the Cline lifecycle integration documented in `hooks/cline/README.md`: `TaskComplete` records a pending review and `UserPromptSubmit` injects this skill's review instruction into the next request. Complete that review before the new request, then run `python3 -B hooks/cline/skill_capture_hook.py acknowledge`, including when no skill is warranted. Do not acknowledge before reviewing. Do not persist the task transcript in hook state. An unattended model turn immediately after completion is not provided by Cline's command-hook API; the next prompt consumes the pending instruction.

## Create or update a skill

1. Determine the intended user outcome, activation scenarios, environment, inputs, and observable completion criteria from the request and repository evidence. Ask only about material unknowns.
2. Read [the format reference](references/skill-format.md) before choosing frontmatter or bundled resources. Prefer the smallest structure that works:

   ```text
   skill-name/
   ├── SKILL.md
   ├── references/   # optional, detailed on-demand documentation
   ├── scripts/      # optional, tested reusable automation
   └── assets/       # optional, templates and output resources
   ```

3. Write imperative, concrete instructions with explicit ordering and decision points. Include verification and safe failure recovery, not merely a description of the domain.
4. Use relative links from the skill root. Refer to a script by its path and show the interpreter when it is not executable. Do not make the agent follow a deep chain of references.
5. For updates, compare old and new behavior. Retain still-valid knowledge, remove contradictions, and add regression coverage when changing a script.
6. Review the description against likely prompts. Include concrete task keywords and triggers without listing unrelated uses.

## Validate and evolve

Run from the repository root:

```bash
python3 skills/skill-creator/scripts/validate_skill.py skills/example-skill
python3 skills/skill-creator/scripts/validate_skill.py skills
python3 -m unittest discover -s skills/skill-creator/tests -v
```

The validator checks portable frontmatter syntax, required metadata, optional fields, the Markdown body, local resource links, credential-like content, and unsafe symlinks. It is a repository convenience check, not proof that every skill client supports a feature or that a procedure works.

When `skills-ref` is available, additionally run `skills-ref validate <skill-dir>` for reference-spec validation. Test scripts with their real dependencies and representative inputs. Treat validation failures as blocking; report unavailable external tools rather than claiming they ran.

Before committing, inspect the final diff and the complete skill pack. Ensure generated skills contain no task transcript, credentials, host-specific paths, or redundant instructions. Commit only when authorized and list every changed file in the final report.
