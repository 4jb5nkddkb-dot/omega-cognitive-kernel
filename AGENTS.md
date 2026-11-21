# Agent workflow

## Skill capture after completed work

Before reporting a project-modifying task complete, review the executed workflow with `skills/skill-creator/SKILL.md`. When it contains a reusable procedure, update the existing matching skill or create a focused skill in `skills/`. Skip one-off work with no transferable procedure and briefly say why. Never capture credentials, private data, unverified claims, or raw task transcripts. Validate the skill pack after any change:

```bash
python3 -B skills/skill-creator/scripts/validate_skill.py skills
```

The Cline hook integration in `hooks/cline/` schedules the same review on `TaskComplete` and delivers pending instructions on the next request. Do not acknowledge a pending review before completing it. The hook does not commit or push changes.
