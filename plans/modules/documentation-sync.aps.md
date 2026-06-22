<!--
APS Module: Documentation Sync
====================================
Keeps docs in sync with feature work. Replaces archived documentation-polish.
See: plans/aps-rules.md
-->

# Documentation Sync

| ID      | Owner | Status      | Progress |
| ------- | ----- | ----------- | -------- |
| DOCSYNC | —     | In Progress | 12/17    |

## Purpose

Keep the docs-site (Docusaurus, sourced from `docs/public/anvil/`) in sync with
feature development: Rust CLI migration, web dashboard rollout, policy
governance, and new language support. API reference generation (from the
OpenAPI spec) is also in scope. Internal-doc governance — ADR process,
architecture diagrams, runbook/as-built freshness — is owned by DOCGOV.

**Problem:** Documentation was polished for 0.1.0 but has no forward plan.
The Rust CLI replaces the Node.js package entirely, the dashboard adds a new
surface, and policy governance changes the governance model — all need
documentation updates that aren't tracked.

## In Scope

DOCSYNC scope is **public-facing Docusaurus content** sourced from
`docs/public/anvil/`. Internal docs under `docs/guides/`, `plans/**`, and
architecture / runbook freshness now live under DOCGOV.

- **Rust migration docs:** Install, CI, troubleshooting updated for native binary
- **Docs-site sync:** Keep public docs in sync with feature releases
- **API reference:** Auto-generate from OpenAPI spec (feeds into API governance)
- **Tutorial updates:** Keep tutorials current as surfaces change (Ink → Ratatui)
- **Multi-version docs:** Support docs for current + previous release

## Out of Scope

- Marketing content (covered by website module)
- Blog posts (separate concern)
- APS specification docs (external repo)
- ADR template/process and architecture diagrams (now owned by DOCGOV)
- Internal runbook and as-built freshness (now owned by DOCGOV-006)

## Interfaces

**Depends on:**

- `docs/public/anvil/` — Docusaurus content source
- `apps/docs-site` — Docusaurus instance (reads from `docs/public/anvil/`)
- Feature modules — source of documentation truth
- API governance — OpenAPI spec for API reference
- DOCGOV-002 — metadata convention every new public doc must carry
- DOCGOV-005 — `pnpm docs:check` validates DOCSYNC output

**Exposes:**

- Public docs-site refresh cadence aligned with feature releases
- Tutorial / quickstart / install-guide surface for the Rust CLI

## Estimated Scope

- **Effort:** 2 weeks

## Work Items

### Rust CLI Migration (0.3.0-beta)

- DOCSYNC-001: Documentation sync checklist per release
- DOCSYNC-002: ADR process documentation and template
- DOCSYNC-003: Architecture diagram update process
- DOCSYNC-004: Tutorial update for Ratatui migration
- DOCSYNC-005: API reference generation pipeline
- DOCSYNC-006: Rust engine architecture documentation
- DOCSYNC-007: Update install, CI, and troubleshooting for native binary
- DOCSYNC-008: Rust migration guide (releases/rust-rewrite.md)
- DOCSYNC-009: Remove Node.js/npm references from all public docs
- DOCSYNC-010: Update beta-testing-guide for 0.3.0-beta

### Future

- DOCSYNC-011: Dashboard feature documentation
- DOCSYNC-012: Policy governance documentation updates
- DOCSYNC-013: Multi-language support documentation
- DOCSYNC-021: Refresh docs for 0.3.2-beta/0.3.3-beta and current repo topology
- DOCSYNC-022: Refresh current public docs for final release scope and 0.4.0-beta watch filtering
- DOCSYNC-023: Full Kindling public docs refresh for upstream 0.2.0 (sibling `eddacraft/kindling`)

### Scanner / Two-Engine State (from RSCAN-008 council review, 2026-04-21)

- DOCSYNC-016: VSCode vs CI warning-divergence troubleshooting entry in `docs/public/anvil/operations/troubleshooting.md`

### Reassigned

- DOCSYNC-014 (Docs contribution guide) → superseded by DOCGOV-001
  (`docs/guides/documentation-governance.md` already covers this)
- DOCSYNC-015, -017, -018, -019, -020 → absorbed and closed in DOCGOV-006
  (targets internal runbook/architecture docs, not the public docs-site)

## Stats

| Phase                           | Total | Done | In Progress | Draft |
| ------------------------------- | ----- | ---- | ----------- | ----- |
| Rust CLI Migration              |    10 |    9 |           0 |     1 |
| Future                          |     6 |    3 |           0 |     3 |
| Scanner / Two-Engine State      |     1 |    0 |           0 |     1 |
| **Total**                       |    17 |   12 |           0 |     5 |

### Item Detail

| ID          | Status | Notes                                         |
| ----------- | ------ | --------------------------------------------- |
| DOCSYNC-001 | Done   | `docs/guides/release-doc-checklist.md` (authority now under DOCGOV per scope sharpening 2026-05-22) |
| DOCSYNC-002 | Done   | `plans/decisions/adr-template.md` + `docs/guides/adr-process.md` (authority now under DOCGOV-004) |
| DOCSYNC-003 | Done   | `docs/guides/architecture-diagrams.md` (authority now under DOCGOV) |
| DOCSYNC-004 | Done   | TUI references updated in beta guide, quickstart |
| DOCSYNC-005 | Draft  |                                               |
| DOCSYNC-006 | Done   | Crate READMEs + rust-rewrite.md               |
| DOCSYNC-007 | Done   | Install, CI, troubleshooting updated across all public docs |
| DOCSYNC-008 | Done   | `docs/public/anvil/releases/rust-rewrite.md`  |
| DOCSYNC-009 | Done   | All `pnpm anvil`/`npx anvil` refs replaced in public docs |
| DOCSYNC-010 | Done   | Beta guide updated for 0.3.0-beta, Node.js dep removed |
| DOCSYNC-011 | Draft  |                                               |
| DOCSYNC-012 | Draft  |                                               |
| DOCSYNC-013 | Draft  |                                               |
| DOCSYNC-016 | Draft  | Origin: operations-reviewer OPS-002 (RSCAN-008 council) |
| DOCSYNC-021 | Done   | 0.3.2/0.3.3 public release docs, auth quickstarts, README and repo-topology docs refreshed |
| DOCSYNC-022 | Done   | Final release-scope pass: current install/upgrade docs + 0.4.0-beta watch-filter docs refreshed |
| DOCSYNC-023 | Done   | Full `docs/public/kindling/` refresh against upstream `eddacraft/kindling` v0.2.0: `demo`/`browse`, thin-client adapters, integrations matrix, VS Code adapter, 0.2 crate versions, retrieval score range, removed stale `list` flags |

### Reassigned items (out of DOCSYNC totals)

| ID          | Disposition                                                                          |
| ----------- | ------------------------------------------------------------------------------------ |
| DOCSYNC-014 | Superseded by DOCGOV-001 (`docs/guides/documentation-governance.md` already covers it) |
| DOCSYNC-015 | Closed by DOCGOV-006 (gate-runner runbook freshness)                                 |
| DOCSYNC-017 | Closed by DOCGOV-006 (`docs/runbooks/release-runbook.md` freshness)                  |
| DOCSYNC-018 | Closed by DOCGOV-006 (`rust-architecture-endstate.md` as-built freshness)            |
| DOCSYNC-019 | Closed by DOCGOV-006 (`docs/guides/release-doc-checklist.md` freshness)              |
| DOCSYNC-020 | Closed by DOCGOV-006 (`docs/guides/anvil-rule-authoring.md` ReDoS framing)           |
