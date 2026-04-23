<!--
APS Module: Documentation Sync
====================================
Keeps docs in sync with feature work. Replaces archived documentation-polish.
See: plans/aps-rules.md
-->

# Documentation Sync

| ID      | Owner | Status      | Progress |
| ------- | ----- | ----------- | -------- |
| DOCSYNC | —     | In Progress | 9/20 (Rust-migration phase 9/10; Future 0/4 Draft; Scanner / Two-Engine 0/6 Draft) |

## Purpose

Keep the docs-site (Docusaurus, sourced from `docs/public/anvil/`) in sync with
feature development: Rust CLI migration, web dashboard rollout, policy
governance, and new language support. Also covers ADR maintenance, architecture
diagram updates, and API reference generation.

**Problem:** Documentation was polished for 0.1.0 but has no forward plan.
The Rust CLI replaces the Node.js package entirely, the dashboard adds a new
surface, and policy governance changes the governance model — all need
documentation updates that aren't tracked.

## In Scope

- **Rust migration docs:** Install, CI, troubleshooting updated for native binary
- **Docs-site sync:** Keep docs in sync with feature releases
- **ADR maintenance:** Ensure new decisions get ADRs, superseded ones are marked
- **Architecture diagrams:** Keep mermaid diagrams current with code changes
- **API reference:** Auto-generate from OpenAPI spec (feeds into API governance)
- **Tutorial updates:** Keep tutorials current as surfaces change (Ink → Ratatui)
- **Multi-version docs:** Support docs for current + previous release
- **Docs contribution guide:** How to update docs when making code changes

## Out of Scope

- Marketing content (covered by website module)
- Blog posts (separate concern)
- APS specification docs (external repo)

## Interfaces

**Depends on:**

- `docs/public/anvil/` — Docusaurus content source
- `apps/docs-site` — Docusaurus instance (reads from `docs/public/anvil/`)
- Feature modules — source of documentation truth
- API governance — OpenAPI spec for API reference

**Exposes:**

- Documentation sync checklist
- ADR template and process
- Architecture diagram update process

## Estimated Scope

- **Effort:** 2 weeks

## Tasks

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
- DOCSYNC-014: Docs contribution guide

### Scanner / Two-Engine State (from RSCAN-008 council review, 2026-04-21)

- DOCSYNC-015: Gate-runner runbook section (CPU/latency envelope, registry.json resolution failure recovery)
- DOCSYNC-016: VSCode vs CI warning-divergence troubleshooting entry in `docs/public/anvil/operations/troubleshooting.md`
- DOCSYNC-017: Name `pnpm test:scanner-parity` as a named preflight gate in `docs/guides/release-runbook.md`
- DOCSYNC-018: Document rayon pool scope / `RAYON_NUM_THREADS` behaviour for `anvil-checks` in `rust-architecture-endstate.md`
- DOCSYNC-019: Extend `docs/guides/release-doc-checklist.md` to include `anvil-rule-authoring.md`, `integrations/vscode.md`, `integrations/mcp.md` for release doc sync
- DOCSYNC-020: Add ReDoS-risk framing for RL-* rule authors (untrusted PR body / commit-message inputs) in `anvil-rule-authoring.md`

## Stats

| Phase                           | Total | Done | In Progress | Draft |
| ------------------------------- | ----- | ---- | ----------- | ----- |
| Rust CLI Migration              |    10 |    9 |           0 |     1 |
| Future                          |     4 |    0 |           0 |     4 |
| Scanner / Two-Engine State      |     6 |    0 |           0 |     6 |
| **Total**                       |    20 |    9 |           0 |    11 |

### Item Detail

| ID          | Status | Notes                                         |
| ----------- | ------ | --------------------------------------------- |
| DOCSYNC-001 | Done   | docs/guides/release-doc-checklist.md (180-line checklist) |
| DOCSYNC-002 | Done   | plans/decisions/adr-template.md + docs/guides/adr-process.md |
| DOCSYNC-003 | Done   | docs/guides/architecture-diagrams.md          |
| DOCSYNC-004 | Done   | TUI references updated in beta guide, quickstart |
| DOCSYNC-005 | Draft  |                                               |
| DOCSYNC-006 | Done   | Crate READMEs + rust-rewrite.md               |
| DOCSYNC-007 | Done   | Install, CI, troubleshooting updated across all public docs |
| DOCSYNC-008 | Done   | docs/public/anvil/releases/rust-rewrite.md    |
| DOCSYNC-009 | Done   | All `pnpm anvil`/`npx anvil` refs replaced in public docs |
| DOCSYNC-010 | Done   | Beta guide updated for 0.3.0-beta, Node.js dep removed |
| DOCSYNC-011 | Draft  |                                               |
| DOCSYNC-012 | Draft  |                                               |
| DOCSYNC-013 | Draft  |                                               |
| DOCSYNC-014 | Draft  |                                               |
| DOCSYNC-015 | Draft  | Origin: operations-reviewer OPS-001 (RSCAN-008 council) |
| DOCSYNC-016 | Draft  | Origin: operations-reviewer OPS-002 (RSCAN-008 council) |
| DOCSYNC-017 | Draft  | Origin: operations-reviewer OPS-003 (RSCAN-008 council) |
| DOCSYNC-018 | Draft  | Origin: operations-reviewer OPS-004 (RSCAN-008 council) |
| DOCSYNC-019 | Draft  | Origin: operations-reviewer OPS-005 (RSCAN-008 council) |
| DOCSYNC-020 | Draft  | Origin: security-analyst NIT #2 (RSCAN-008 council)    |
