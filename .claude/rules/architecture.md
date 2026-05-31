# Architecture Decisions

Before proposing architectural changes, read
`plans/decisions/DECISION-LOG.md` for existing decisions and
`docs/vision/anvil-scope-guard.md` for scope boundaries.

Key principles (all features must align):
- **Deterministic** — same input, same output
- **Warnings over blocks** — exit 0 by default
- **New edges only** — baseline existing state, warn on new violations

Anvil's zero-config product posture ("planless-first", ADR-001) describes the
tool's UX — value without writing a plan — and is not a per-feature evaluation
gate. It marks the pivot away from spec-driven validation (validating code
against a plan); don't use it to flag or block feature work.

When making a new architectural decision, follow `docs/guides/adr-process.md`
and add the entry to `plans/decisions/DECISION-LOG.md`.
