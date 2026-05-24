# Continuous Improvement Log

This file captures lightweight session learning from agents. It is evidence, not
a backlog. Promote repeated friction or executable follow-up work to
`plans/modules/continuous-improvement-backlog.aps.md` as `CIB-NNN` items.

## Template

```md
### YYYY-MM-DD — opencode|claude|other

- **Task:** ...
- **Outcome:** ...
- **Worked:** ...
- **Failed:** ...
- **Friction:** ...
- **Improvement:** ...
- **Follow-up:** ...
```

## Entries

### 2026-05-24 — opencode

- **Task:** Add continuous-improvement closeout to repo-local dev-workflow skills.
- **Outcome:** Added explicit trigger contracts, continuous-improvement closeout
  rules, and this shared log.
- **Worked:** The active CIB APS module already existed, so the log could stay
  evidence-only instead of becoming a second backlog.
- **Failed:** Nothing substantive.
- **Friction:** The Claude skill description was much less explicit than the
  OpenCode copy, which likely contributed to agents skipping it.
- **Improvement:** Keep mandatory skill triggers concrete in frontmatter and
  repeat the trigger contract near the top of the skill body.
- **Follow-up:** Watch whether Claude still skips dev-workflow after restart; if
  yes, add a global skill or command-level reminder outside this repo-local copy.

### 2026-05-24 — opencode

- **Task:** Prepare PR for dev-workflow continuous-improvement closeout.
- **Outcome:** Quick Council caught a read-only-task edge case before PR
  publication.
- **Worked:** Running a scoped Council pass before opening the PR found a
  workflow-rule conflict that normal markdown validation would not catch.
- **Failed:** Initial wording made the continuous-improvement note mandatory even
  when a task explicitly forbids writes.
- **Friction:** The OpenCode skill surface inventory had drifted from the
  repo-local `.opencode/skills/` directories.
- **Improvement:** Mandatory closeout rules need explicit no-write exceptions,
  and skill inventories should be checked against the filesystem when touched.
- **Follow-up:** none

### 2026-05-25 — claude

- **Task:** Ship MLP2-051g (`anvil start --verify --why` verbose
  tier-evidence flag) via /dev-workflow.
- **Outcome:** PR #1909 opened; 7 new pinned tests; full workspace
  cargo test + clippy + fmt clean.
- **Worked:** APS Truth Gate caught that the spec's design was
  already fully locked by `plans/specs/2026-05-21-activation-daemon-evidence-wireup.md`
  §"Council Verdicts" item 9, so no brainstorming or planning-council
  pass was needed before code. The MLP2-070 "validate prod wire-up
  before branching" check also ran clean (no stealth implementation).
- **Failed:** Initial `render_human_verbose` missed `McpTier::NotDetected`
  — the match was non-exhaustive and rustc caught it. Cheap.
- **Friction:** `cargo check -p eddacraft-anvil-cli` failed with
  "did not match any packages" because the crate name in
  `crates/anvil-cli/Cargo.toml` is `eddacraft-anvil`, not
  `eddacraft-anvil-cli`. The directory/crate-name divergence trips
  every fresh session.
- **Improvement:** none — the divergence is a one-off learnt-once
  fact; not worth a CIB entry.
- **Follow-up:** Run /addressing-pr-reviews on #1909 after CI + Copilot
  settle.
