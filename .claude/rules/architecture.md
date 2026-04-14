# Architecture Decisions

Before proposing architectural changes, read
`plans/decisions/DECISION-LOG.md` for existing decisions and
`docs/vision/anvil-scope-guard.md` for scope boundaries.

Key principles (all features must align):
- **Planless-first** — value without requiring config or plans
- **Deterministic** — same input, same output
- **Warnings over blocks** — exit 0 by default
- **New edges only** — baseline existing state, warn on new violations

When making a new architectural decision, follow `docs/guides/adr-process.md`
and add the entry to `plans/decisions/DECISION-LOG.md`.
