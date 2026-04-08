# Post-Merge Test Plan Template

Copy this file to `plans/reviews/post-merge/<branch-slug>.md` when opening a PR
that includes post-merge verification steps.

The cleanup agent will pick this up after merge and attempt to verify/execute
each step. Steps that require human action will be flagged in `cleanup-log.md`
and sent as a Telegram notification.

---

# Post-merge: <branch-slug>

PR: #NNN
Branch: `<branch-slug>`
APS: MODULE-NNN
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — description (agent: yes/no)
- [ ] Step 2 — description (agent: yes/no)
- [ ] Step 3 — description (human required)

## Notes

Any context the cleanup agent needs to verify steps correctly.

---

## Field Guide

| Field | Notes |
|---|---|
| `PR` | GitHub PR number |
| `Branch` | Branch slug matching the filename |
| `APS` | Module ID to advance to Complete when all steps verified |
| `Merged` | Filled automatically by cleanup agent on detection |
| `Verified` | Filled automatically when all agent-verifiable steps pass |
| `(agent: yes)` | Cleanup agent can verify this step automatically |
| `(agent: no)` / `(human required)` | Cleanup agent will flag for human attention |
