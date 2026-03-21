<!--
APS Module: Documentation Sync
====================================
Keeps docs in sync with feature work. Replaces archived documentation-polish.
See: plans/aps-rules.md
-->

# Documentation Sync

| ID      | Owner | Status    |
| ------- | ----- | --------- |
| DOCSYNC | —     | Draft |

## Purpose

Keep the docs-site (Docusaurus) in sync with feature development: Rust engine
migration, web dashboard rollout, policy governance, and new language support.
Also covers ADR maintenance, architecture diagram updates, and API reference
generation.

**Problem:** Documentation was polished for 0.1.0 but has no forward plan.
The Rust engine changes the architecture story significantly, the dashboard
adds a new surface, and policy governance changes the governance model —
all need documentation updates that aren't tracked.

## In Scope

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

- `apps/docs-site` — Docusaurus instance
- Feature modules — source of documentation truth
- API governance — OpenAPI spec for API reference

**Exposes:**

- Documentation sync checklist
- ADR template and process
- Architecture diagram update process

## Estimated Scope

- **Effort:** 1 week

## Tasks

- DOCSYNC-001: Documentation sync checklist per release
- DOCSYNC-002: ADR process documentation and template
- DOCSYNC-003: Architecture diagram update process
- DOCSYNC-004: Tutorial update for Ratatui migration
- DOCSYNC-005: API reference generation pipeline
- DOCSYNC-006: Rust engine architecture documentation
