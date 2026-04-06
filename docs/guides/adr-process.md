# ADR Process

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

1. Copy the template:
   `cp plans/decisions/adr-template.md plans/decisions/NNN-short-title.md`
2. Use the next available number (check existing files in `plans/decisions/`)
3. Use kebab-case for the filename: `NNN-short-description.md`
4. Set status to **Proposed**
5. Fill in all sections — Context and Decision are mandatory; Alternatives
   Considered is strongly encouraged
6. Commit the ADR alongside the code it relates to, or in the planning PR

## Numbering

- Sequential three-digit numbers: `000`, `001`, ..., `016`
- Suffix variants with a letter: `011a` (for a follow-up to ADR-011)
- No gaps — if an ADR is rejected, keep the number and set status to Rejected

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
