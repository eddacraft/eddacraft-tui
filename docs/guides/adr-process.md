# ADR Process

| Type  | Authority     | Owner  | Status | Freshness                                                                                                        |
| ----- | ------------- | ------ | ------ | ---------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DOCGOV | Live   | Last reviewed 2026-08-17 against `scripts/docs/adr-integrity.sh`, ADR-123, and `plans/decisions/DECISION-LOG.md` |

| Upstream                                                                                              | Downstream                                                                   |
| ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `plans/decisions/adr-template.md`, `plans/decisions/DECISION-LOG.md`, `scripts/docs/adr-integrity.sh` | `AGENTS.md`, `plans/decisions/`, `pnpm adr:check`, `pnpm test:adr-integrity` |

Architecture Decision Records (ADRs) capture significant technical decisions
with their context and consequences. They live in `plans/decisions/` and are
numbered sequentially.

## When to Write an ADR

Write an ADR when:

- Choosing between competing approaches with real trade-offs
- Making a decision that is hard or expensive to reverse
- Overriding or superseding a previous decision
- Adopting a new technology, framework, or pattern
- Establishing a convention the team should follow

Do **not** write an ADR for:

- Obvious choices with no real alternative
- Implementation details that are easy to change
- Bug fixes or routine refactoring

## How to Create an ADR

1. Run `pnpm adr:check` against the live repo to confirm the log is currently
   clean and to read the next-available ADR number printed by the script
   (`pnpm test:adr-integrity` runs the fixture tests against sandbox trees, not
   the real `plans/decisions/`, so it won't surface a real next-available
   number)
2. Copy the template:
   `cp plans/decisions/adr-template.md plans/decisions/NNN-short-title.md`
3. Use the next available number; if you raced another author and they took it
   first, **renumber yours before merge** and update any references — do not
   introduce a duplicate
4. Use kebab-case for the filename: `NNN-short-description.md`
5. Set status to **Proposed** (or **Draft** for in-flight ideas not yet ready
   for council/PR review)
6. Fill in all sections — Context and Decision are mandatory; Alternatives
   Considered is strongly encouraged
7. Add the ADR row to the appropriate section of
   [`plans/decisions/DECISION-LOG.md`](../../plans/decisions/DECISION-LOG.md) in
   the same PR
8. Commit the ADR alongside the code it relates to, or in the planning PR

## Numbering

- Sequential three-digit numbers, zero-padded: `000`, `001`, …, `041` …
- Suffix variants with a letter: `011a` (for a follow-up to ADR-011)
- **No gaps** — if an ADR is rejected, keep the number and set status to
  Rejected. If a gap exists from a historical race, the next renumber-on- rename
  event should backfill it (see ADR-021 / ADR-026 history in the decision log
  for an example).
- **No duplicate numbers** — `pnpm adr:check` (run against the live repo) and
  `pnpm test:adr-integrity` (fixture tests for the script itself) both fail if
  two ADR files share a number, or if an ADR file is not indexed in
  `DECISION-LOG.md`, or vice versa.

## Status Lifecycle

```
Draft → Proposed → Accepted
                 → Rejected
       Accepted  → Superseded (link to replacement)
```

- **Draft:** Work in progress, not yet ready for review
- **Proposed:** Ready for review — include in a PR or council session
- **Accepted:** Decision is in effect
- **Rejected:** Decision was considered but not adopted (keep for context)
- **Superseded:** Replaced by a newer decision (add a note linking to the
  replacement at the top of the file)

## Review

ADRs are reviewed through the normal PR process or during Council sessions for
high-impact decisions. The reviewer should check:

- Context explains the problem clearly
- Decision is specific and actionable
- Alternatives were genuinely considered
- Consequences are honest about trade-offs

## Referencing ADRs

- In code comments: `// See ADR-016 for rationale`
- In APS plans: link to the file or reference by number
- In commit messages: `Implements ADR-016` or `Relates to ADR-016`
