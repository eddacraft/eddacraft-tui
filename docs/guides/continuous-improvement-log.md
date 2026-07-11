# Continuous Improvement Log

| Type  | Authority     | Owner | Status | Freshness                                       |
| ----- | ------------- | ----- | ------ | ----------------------------------------------- |
| Guide | Authoritative | CIB   | Live   | 2026-07-12 — pending-queue durability (CIB-191) |

| Upstream                                                                                                                    | Downstream                                                      |
| --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `plans/reviews/continuous-improvement-log.md`, `plans/modules/continuous-improvement-backlog.aps.md`, `dev-workflow` skills | Agents closing sessions, bookkeeping harvest PRs, weekly triage |

## Purpose

Capture **session evidence** (what worked, what failed, what friction remains)
without turning it into a second backlog, and without forcing every feature PR
to carry an "unrelated" log file that agents then omit — and lose.

## Two surfaces

| Surface           | Path                                                                       | Role                                                                        |
| ----------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| **Pending queue** | `.git/anvil/ci-log-pending/*.md` (git common dir; shared across worktrees) | Default write target. Survives worktree removal. Not in `git status`.       |
| **Tracked log**   | `plans/reviews/continuous-improvement-log.md` (`merge=union`)              | Durable evidence on `main`. Harvested from pending on bookkeeping branches. |
| **Backlog**       | `plans/modules/continuous-improvement-backlog.aps.md`                      | Executable `CIB-NNN` work after triage.                                     |

Do **not** file GH issues for process hygiene that belongs in CIB. Do **not**
use `plans/issues.md` for session friction.

## Why pending exists

Historical failure mode (2026-07): agents appended to the tracked log in a
worktree, then opened a single-purpose feature PR and left the log out because
it looked unrelated. Worktree cleanup discarded the uncommitted note. Result:
~100 PRs over several days with almost no log growth.

The pending queue breaks that coupling:

```text
session closeout → pnpm ci-log:append (pending)
                 → worktree / PR can ignore the log
bookkeeping      → pnpm ci-log:harvest → commit tracked log
weekly triage    → pnpm ci-log:since -- --watermark → promote/absorb/leave
                 → pnpm ci-log:set-watermark -- --today
```

## Agent closeout (every non-trivial session)

1. Prefer:

   ```bash
   pnpm ci-log:append -- \
     --agent opencode|claude|codex \
     --task "..." \
     --outcome "..." \
     --worked "..." \
     --failed "none" \
     --friction "..." \
     --improvement "none|..." \
     --follow-up "none|session:...|promote: CIB|theme:...|owned: ID"
   ```

2. Or pass a full entry:

   ```bash
   pnpm ci-log:append -- --stdin <<'MD'
   ### 2026-07-12 — opencode

   - **Task:** ...
   - **Outcome:** ...
   - **Worked:** ...
   - **Failed:** none
   - **Friction:** none
   - **Improvement:** none
   - **Follow-up:** none
   MD
   ```

3. If the task contract forbids writes, skip and say so in the final response.
4. `Improvement: none` is valid — do not invent filler.
5. **Do not** require the tracked log in a feature PR. Pending is enough.
6. Optional: `--tracked` when the PR is already bookkeeping/docs and you want
   the note on the branch immediately.

### Follow-up vocabulary

| Value               | Meaning                                |
| ------------------- | -------------------------------------- |
| `none`              | No durable next step                   |
| `session:...`       | Branch-local next step; do not promote |
| `promote: CIB`      | Ready for a CIB item at next triage    |
| `theme:name`        | Cluster key for recurrence             |
| `owned: MODULE-NNN` | Already tracked                        |

## Harvest (bookkeeping)

When pending count is non-zero (see `pnpm ci-log:status` or session-start
banner):

```bash
pnpm ci-log:status
pnpm ci-log:harvest
git add plans/reviews/continuous-improvement-log.md
# commit on docs/* or chore/* bookkeeping branch; open PR
```

Harvest may be combined with APS status bookkeeping. Do not block feature work
on harvest.

## Weekly triage

1. `pnpm ci-log:harvest` if anything is pending (or note that harvest PR is
   open).
2. `pnpm ci-log:since -- --watermark` — review entries since last triage.
3. For each (or each theme): **promote** (file `CIB-NNN`), **absorb** (already
   owned — leave a one-line note in the triage CI-log entry), or **leave**
   (one-off lesson).
4. Promotion bar: intent + observable outcome + validation + source pointer.
5. `pnpm ci-log:set-watermark -- --today`.
6. Append one triage closeout note via `pnpm ci-log:append` (or tracked).

Workflow: `.claude/workflows/triage-ci-log.js` when running under Claude
workflows.

## Commands

| Command                     | Purpose                                |
| --------------------------- | -------------------------------------- |
| `pnpm ci-log:append`        | Write pending (default) or `--tracked` |
| `pnpm ci-log:harvest`       | Pending → tracked log                  |
| `pnpm ci-log:status`        | Pending count, last entry, watermark   |
| `pnpm ci-log:since`         | Entries since date or watermark        |
| `pnpm ci-log:set-watermark` | Update triage watermark                |
| `pnpm test:ci-log`          | Fixture tests                          |

## Related

- Tracked log:
  [`plans/reviews/continuous-improvement-log.md`](../../plans/reviews/continuous-improvement-log.md)
- Backlog:
  [`plans/modules/continuous-improvement-backlog.aps.md`](../../plans/modules/continuous-improvement-backlog.aps.md)
- Project rules: [`plans/project-context.md`](../../plans/project-context.md)
- Skills: `dev-workflow` (Claude / OpenCode / Codex)
