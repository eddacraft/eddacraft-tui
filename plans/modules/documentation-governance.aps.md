<!--
APS Module: Documentation Governance
====================================
Defines the operational knowledge architecture for engineering docs and agent
closeout behaviour. See: plans/aps-rules.md
-->

# Documentation Governance

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| DOCGOV | —     | In Progress | 1/8      |

## Purpose

Create a coherent operational knowledge system for AI-native software
engineering without turning documentation into compliance theatre.

## In Scope

- Documentation authority model across APS, ADRs, as-built docs, runbooks,
  guides, public docs, and package READMEs
- Agent-facing documentation workflow and mandatory closeout behaviour
- Lifecycle metadata and freshness conventions for operational docs
- Validation rules for links, ownership, status, ADR integrity, APS consistency,
  and as-built source references
- Migration path from today's mixed documentation styles to a smaller governed
  model

## Cross-Cutting Convention

This is a cross-cutting APS module and follows the rules in
[`plans/aps-rules.md#cross-cutting-modules`](../aps-rules.md#cross-cutting-modules).
Task closeout must sweep `Coordinates with:`, `Blocks on:`, `Supersedes:`, and
`Superseded by:` callouts rather than carrying unresolved references into
archive.

## Out of Scope

- Replacing APS as execution authority
- Rewriting all existing documentation in one pass
- Moving folders before validation rules exist
- Creating process-only documents that cannot be enforced or checked

## Interfaces

**Depends on:**

- `plans/aps-rules.md` — APS structure, lifecycle, and agent rules
- `plans/decisions/DECISION-LOG.md` — architecture decision index
- `docs/architecture/README.md` — current architecture taxonomy seed
- `docs/architecture/_as-built-template.md` — as-built documentation pattern
- `AGENTS.md` — repository-wide agent behaviour rules
- `plans/specs/2026-05-09-plan-build-release-operating-model.md` — target
  operating lifecycle and source-of-truth hierarchy
- `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md` — skill,
  agent, hook, session, and event authority boundaries

**Exposes:**

- Documentation authority model
- Documentation workflow and closeout protocol
- Validation backlog for documentation integrity
- Migration plan for taxonomy, metadata, and generated indexes

## Coherence Boundary

DOCGOV owns documentation classification, authority routing, closeout, metadata,
and validation. It does not own the Plan / Build / Release lifecycle, release
state semantics, or agent execution taxonomy. When documentation governance needs
those concepts, it links to the operating-model and agentic-execution specs
instead of redefining them.

Current planned validation commands such as `pnpm docs:check`,
`pnpm docs:index`, and `pnpm docs:index:check` are target commands until
DOCGOV-005 and DOCGOV-007 implement them. Before those items land, docs changes
use the minimal validation baseline in
`docs/guides/documentation-governance.md`.

## Tasks

### DOCGOV-001: Establish documentation workflow and closeout rules

- **Status:** Complete
- **Intent:** Define how agents classify documentation work and complete the
  hygiene steps that usually get skipped at the end.
- **Expected Outcome:** Repository guidance names the authoritative sources,
  documents a docs-workflow skill shape, and requires closeout before final
  responses on documentation-affecting work.
- **Validation:** `pnpm format:check`
- **Files:** `AGENTS.md`, `docs/guides/documentation-governance.md`,
  `docs/guides/README.md`, `docs/README.md`, `plans/index.aps.md`,
  `plans/modules/documentation-governance.aps.md`
- **Coordinates with:** Closed for DOCGOV-001. Follow-up coordination with
  DOCSYNC, MDGOV, ADR process, as-built documentation, release runbooks, and
  package/crate README maintenance is captured in DOCGOV-002..DOCGOV-008.
- **Confidence:** high

### DOCGOV-002: Canonicalise documentation taxonomy and metadata

- **Status:** Ready
- **Intent:** Add a minimal document taxonomy and metadata convention without
  forcing a large folder migration.
- **Expected Outcome:** New or touched docs can declare type, authority, owner,
  status, freshness, and upstream/downstream references consistently.
- **Validation:** `pnpm format:check`
- **Dependencies:** DOCGOV-001
- **Confidence:** high

### DOCGOV-003: Align APS public docs, local rules, and schemas

- **Status:** Proposed
- **Intent:** Remove contradictions between public APS docs, local APS rules,
  package schemas, parser expectations, and current repository usage.
- **Expected Outcome:** APS status vocabulary, file layout, task headings,
  validation claims, and package README links describe the same executable
  contract.
- **Validation:** `pnpm -F @eddacraft/anvil-aps test && pnpm format:check`
- **Dependencies:** DOCGOV-002
- **Confidence:** medium

### DOCGOV-004: Repair ADR integrity and enforcement

- **Status:** Proposed
- **Intent:** Make ADR numbering, lifecycle, and decision-log coverage
  mechanically trustworthy.
- **Expected Outcome:** Duplicate/missing ADR index entries are resolved, the ADR
  process guide matches the current repository, and a validation path exists for
  future ADR changes.
- **Validation:** `pnpm format:check`
- **Dependencies:** DOCGOV-002
- **Confidence:** high

### DOCGOV-005: Add documentation validation baseline

- **Status:** Proposed
- **Intent:** Convert closeout from memory-based hygiene into fully automated
  checks.
- **Expected Outcome:** `pnpm docs:check` validates metadata, tags, links,
  APS/index consistency, ADR integrity, generated-index freshness, and as-built
  source path existence. Manual indexing is not allowed; the only manual input is
  document-local metadata and approved tag catalogue updates. Until this ships,
  references to `pnpm docs:check` are target-state guidance, not an available
  repository command.
- **Validation:** `pnpm format:check && pnpm lint:check`
- **Dependencies:** DOCGOV-002, DOCGOV-004
- **Confidence:** medium

### DOCGOV-006: Standardise runbook and as-built freshness

- **Status:** Proposed
- **Intent:** Ensure operational docs expose owner, scope, verification date,
  source references, and stale-state signals.
- **Expected Outcome:** Runbook and as-built templates define required freshness
  metadata, and representative docs are migrated as examples.
- **Validation:** `pnpm format:check`
- **Dependencies:** DOCGOV-002, DOCGOV-005
- **Confidence:** medium

### DOCGOV-007: Generate or reconcile documentation indexes

- **Status:** Proposed
- **Intent:** Replace hand-maintained documentation discovery with generated
  indexes.
- **Expected Outcome:** `pnpm docs:index` generates indexes by type, authority,
  owner, status, and tag from document metadata; `pnpm docs:index:check` fails CI
  when generated indexes are stale. New tags are added through the approved tag
  catalogue, not by manually editing indexes. Until this ships, generated-index
  requirements remain planned and must not be treated as current closeout
  commands.
- **Validation:** `pnpm format:check && pnpm lint:check`
- **Dependencies:** DOCGOV-005
- **Confidence:** medium

### DOCGOV-008: Migrate stale entrypoints and archive dead docs

- **Status:** Proposed
- **Intent:** Reduce ambiguity by fixing stale onboarding links and moving dead
  operational docs out of active paths.
- **Expected Outcome:** Contributor entrypoints route through current indexes,
  stale specs are marked or archived, the `docs/guides/release-runbook.md`
  migration exception is resolved, and public/internal docs platform claims are
  reconciled.
- **Validation:** `pnpm format:check`
- **Dependencies:** DOCGOV-005, DOCGOV-007
- **Confidence:** medium
