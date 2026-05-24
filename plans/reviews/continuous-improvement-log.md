# Continuous Improvement Log

This file captures lightweight session learning from agents. It is evidence, not
a backlog. Promote repeated friction or executable follow-up work to
`plans/modules/continuous-improvement-backlog.aps.md` as `CIB-NNN` items.

## Template

```md
<ci-log date="YYYY-MM-DD" agent="opencode|claude|other">
task:
outcome:
worked:
failed:
friction:
improvement:
follow-up:
</ci-log>
```

## Entries

<ci-log date="2026-05-24" agent="opencode">
task: Add continuous-improvement closeout to repo-local dev-workflow skills.
outcome: Added explicit trigger contracts, CI-note closeout rules, and this shared log.
worked: The active CIB APS module already existed, so the log could stay evidence-only instead of becoming a second backlog.
failed: Nothing substantive.
friction: The Claude skill description was much less explicit than the OpenCode copy, which likely contributed to agents skipping it.
improvement: Keep mandatory skill triggers concrete in frontmatter and repeat the trigger contract near the top of the skill body.
follow-up: Watch whether Claude still skips dev-workflow after restart; if yes, add a global skill or command-level reminder outside this repo-local copy.
</ci-log>
