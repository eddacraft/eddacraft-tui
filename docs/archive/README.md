# Documentation Archive

Historical documents preserved for reference. **Do not update these.**

## Contents

### edda-pre-implementation/

EDDA architecture docs written before implementation. Superseded by the actual
code in `packages/edda-stack/`. Includes the system architecture (2300+ lines),
component dependency analysis, storage comparison, and planning index.

### Historical Planning (`planning/`)

Past planning documents from the 2026-01-18 docs cleanup: V1 feature alignment,
monorepo planning, LSP/TUI implementation plans, roadmaps.

### Historical Status (`status/`)

Past status reports and milestone summaries.

### Architecture review (2026-08-05)

| File | Why archived |
| ---- | ------------ |
| `architecture/anvil-full-architecture.md` | Pre-cutover CURRENT/PROPOSED synthesis (2026-03-13); mislabelled shipped Rust runtime as proposed. Live authority: `docs/architecture/overview.md` + `*-as-built.md` |
| `architecture/rust-architecture-endstate.md` | Aspirational H1/H2 end-state (2026-04-03) with outdated module statuses. Live authority: `docs/architecture/rust-architecture-overview.md` + as-builts |

### DOCGOV-008 Archive Moves

Documents archived on 2026-05-23 during DOCGOV-008 because they were stale,
superseded by implementation, or historical planning artefacts with current
authority elsewhere.

| File | Why archived |
| ---- | ------------ |
| `architecture/monorepo-structure.md` | Historical monorepo migration plan; current structure lives in source and architecture indexes |
| `guides/first-rust-release-rehearsal.md` | Draft rehearsal for a shipped Rust release line |
| `guides/rust-cli-release-scope.md` | v0.3.x Rust CLI scope note superseded by v0.7.x release state |
| `marketing/anvil-product-sheet.md` | Moment-in-time marketing sheet with no live referrers |
| `plans/2026-03-09-aps-vs-gh-projects-trial-decision-space.md` | APS-vs-GitHub-Projects decision space after APS became system of record |
| `plans/2026-03-11-verifiable-governance-technical-design.md` | Design planning replaced by shipped code, APS, and governance validators |
| `plans/2026-03-17-lineage-authorship-confidence-v1.md` | v1 lineage/authorship design superseded by current APS/code authority |
| `reviews/deep-research-report.md` | One-off review snapshot with follow-ups either landed or tracked elsewhere |
| `runbooks/v0.6.0-beta-release-runbook.md` | Historical v0.6.0-beta release runbook superseded by v0.7.x release runbooks |
| `runbooks/v0.6.0-beta-security-note.md` | Historical v0.6.0-beta security note superseded by v0.7.x release notes |
| `runbooks/v0.6.x-to-v0.7.0-beta-migration.md` | Migration guidance for the previous v0.6.x line, retained as history |
| `specs/2026-03-12-product-licensing-design.md` | Historical licensing design superseded by active plans and implementation |
| `specs/2026-03-15-beta-auth-streamline-design.md` | Historical beta auth design; current docs auth work lives in plans/specs |
| `specs/2026-03-18-pitch-deck-direction-design.md` | Pitch-deck direction snapshot after deck production moved on |
| `specs/2026-03-27-rust-cli-cutover-design.md` | Rust CLI cutover design after the cutover shipped |
| `specs/command-safety-validation.md` | Draft command-safety spec superseded by current validation implementation |
| `specs/edda-api-contracts.md` | Pre-Anvil Edda draft with no live authority |
| `specs/edda-authority-trust.md` | Pre-Anvil Edda draft with no live authority |
| `specs/edda-enforcement-hooks.md` | Pre-Anvil Edda draft with no live authority |

### Individual Files

| File                                   | Why archived                                   |
| -------------------------------------- | ---------------------------------------------- |
| `rust-kernel-post-h1.md`               | Post-H1 future reference, not in current scope |
| `kindling-integration-analysis.md`     | Decision made, integration exists              |
| `diagram-rendering-for-ratatui.md`     | Research for future TUI work                   |
| `node-to-deno-migration-assessment.md` | Assessment complete, no action taken           |
| `claude-guide-legacy.md`               | Superseded by root `CLAUDE.md`                 |
| `git-worktree-workflow.md`             | Generic git education, not Anvil-specific      |
| `bmad-adapter-spec.md`                 | BMAD adapter complete, spec lives in code      |
| `test-quality-patterns-proposal.md`    | Proposed antipatterns, not yet implemented     |

## Current Documentation

- **Planning** → `plans/` (root directory)
- **Engineering docs** → `docs/` (parent directory)
- **Public docs** → `docs/public/`
