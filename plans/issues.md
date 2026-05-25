<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->

# Anvil — Planning Issues and Questions

> **Purpose:** capture development-time discoveries that surface during APS work
> without forcing every observation into a full module spec. This file is the
> canonical APS issues/questions surface (`ISS-NNN`, `Q-NNN`).
>
> **Not** for production incidents, routine bug reports, GitHub Issues, or
> ad-hoc decision logging — those have their own homes:
>
> - GitHub issues for product bugs and tracked feature requests.
> - `plans/decisions/DECISION-LOG.md` for ADR-style decisions.
> - `plans/reviews/continuous-improvement-log.md` for per-session observations.
> - `plans/modules/continuous-improvement-backlog.aps.md` (CIB) for promoted
>   recurring friction.

## Authority

| Type  | Authority     | Owner  | Status | Freshness                                  |
| ----- | ------------- | ------ | ------ | ------------------------------------------ |
| Index | Authoritative | APSCAN | Live   | Created 2026-05-25 for canonical alignment |

| Upstream                                                | Downstream                                       |
| ------------------------------------------------------- | ------------------------------------------------ |
| `plans/aps-rules.md`, `plans/project-context.md`, agents | `plans/modules/*.aps.md` when items are promoted |

## How to use

1. **Issues (`ISS-NNN`)** — bugs, limitations, or edge cases noticed during
   APS-authoring or APS-validated work that do not yet have an owning module
   work item.
2. **Questions (`Q-NNN`)** — unknowns or deferred decisions that block future
   APS work. When answered, the answer either resolves the question inline or
   promotes the work into a module.
3. **Promotion path** — when an issue or question grows into bounded work, file
   it as a `MODULE-NNN` work item in the appropriate `plans/modules/*.aps.md`
   and mark the row here **promoted**. Keep the row for traceability.
4. **Resolution** — when an issue is fixed or a question is answered without
   promotion, mark the row **resolved** with a one-line evidence pointer.
5. **Numbering** — sequential per type. Do not renumber; gaps are fine.

## Status vocabulary

| Status     | Meaning                                                          |
| ---------- | ---------------------------------------------------------------- |
| `open`     | Logged, not yet acted on.                                        |
| `promoted` | Filed as a module work item; row kept for traceability.          |
| `resolved` | Addressed without a module work item; evidence link required.    |
| `wontfix`  | Deliberately not actioned; one-line rationale required.          |

## Issues (ISS-NNN)

| ID | Status | Summary | Source / Evidence |
| -- | ------ | ------- | ----------------- |
| _none yet_ | — | First `ISS-NNN` will be filed during canonical-alignment migration waves. | — |

## Questions (Q-NNN)

| ID | Status | Question | Resolution / Evidence |
| -- | ------ | -------- | --------------------- |
| _none yet_ | — | First `Q-NNN` will be filed when an APSCAN sub-task surfaces an open design question. | — |

## Notes

- This file is allow-listed in `.gitignore`'s `plans/reviews/*` block by being
  outside that block — it lives at `plans/issues.md`, not under `plans/reviews/`.
- Generated canonical context packages live under `.aps/context/<ID>.md` and
  are git-ignored; treat them as safe-to-regenerate cache output.
- When the issue/question count grows large enough to need querying, add a
  drift-check pass that validates ID format and status values. Not warranted at
  zero rows.
