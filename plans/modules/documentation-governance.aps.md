<!--
APS Module: Documentation Governance
====================================
Defines the operational knowledge architecture for engineering docs and agent
closeout behaviour. See: plans/aps-rules.md
-->

# Documentation Governance

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| DOCGOV | —     | In Progress | 9/12     |

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
- Backfilling the DOCGOV-002 metadata format onto pre-existing live documents
- Reorganising live documents under the canonical taxonomy once the metadata
  and validation baseline are in place

## Cross-Cutting Convention

This is a cross-cutting APS module and follows the rules in
[`plans/aps-rules.md#cross-cutting-modules`](../aps-rules.md#cross-cutting-modules).
Task closeout must sweep `Coordinates with:`, `Blocks on:`, `Supersedes:`, and
`Superseded by:` callouts rather than carrying unresolved references into
archive.

## Out of Scope

- Replacing APS as execution authority
- Rewriting all existing documentation in one pass
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

Current validation commands include `pnpm docs:check`, `pnpm docs:index`, and
`pnpm docs:index:check`. DOCGOV-006 and DOCGOV-007 have replaced the planned
`asbuilt-paths` and `index-freshness` stubs with real validators.

## Tasks

### DOCGOV-001: Establish documentation workflow and closeout rules

- **Status:** Complete
- **Intent:** Define how agents classify documentation work and complete the
  hygiene steps that usually get skipped at the end.
- **Expected Outcome:** Repository guidance names the authoritative sources,
  documents a docs-workflow skill shape, and requires closeout before final
  responses on documentation-affecting work.
- **Validation:** `pnpm format:check`; manual metadata, index, and APS
  reconciliation review
- **Files:** `AGENTS.md`, `docs/guides/documentation-governance.md`,
  `docs/guides/README.md`, `docs/README.md`, `plans/index.aps.md`,
  `plans/modules/documentation-governance.aps.md`
- **Coordinates with:** Closed for DOCGOV-001. Follow-up coordination with
  DOCSYNC, MDGOV, ADR process, as-built documentation, release runbooks, and
  package/crate README maintenance is captured in DOCGOV-002..DOCGOV-008.
- **Closeout:** Validation passed with `pnpm format:check`; manual metadata,
  index, and APS reconciliation review performed ahead of merge.
- **Confidence:** high

### DOCGOV-002: Canonicalise documentation taxonomy and metadata

- **Status:** Complete
- **Intent:** Add a minimal document taxonomy and metadata convention without
  forcing a large folder migration.
- **Expected Outcome:** New or touched docs can declare type, authority, owner,
  status, freshness, and upstream/downstream references consistently.
- **Validation:** `pnpm format:check`
- **Dependencies:** DOCGOV-001
- **Files:** `docs/guides/documentation-governance.md`,
  `docs/architecture/_as-built-template.md`, `docs/guides/README.md`,
  `docs/README.md`, `plans/modules/documentation-governance.aps.md`,
  `plans/index.aps.md`
- **Closeout:** Validation passed with `pnpm format:check`; council findings on
  metadata applicability, required fields, status vocabulary, as-built document
  status, and README freshness anchors were addressed before PR.
- **Confidence:** high

### DOCGOV-003: Align APS public docs, local rules, and schemas

- **Status:** Complete
- **Intent:** Remove contradictions between public APS docs, local APS rules,
  package schemas, parser expectations, and current repository usage.
- **Expected Outcome:** APS status vocabulary, file layout, task headings,
  validation claims, and package README links describe the same executable
  contract.
- **Validation:** `pnpm -F @eddacraft/anvil-aps test && pnpm format:check`
- **Dependencies:** DOCGOV-002
- **Files:** `plans/aps-rules.md`, `packages/aps/README.md`,
  `packages/aps/AGENTS.md`, `plans/modules/documentation-governance.aps.md`,
  `plans/index.aps.md`
- **Closeout:** Validation passed with
  `pnpm -F @eddacraft/anvil-aps test && pnpm format:check` ahead of merge.
  `plans/aps-rules.md` now distinguishes the five schema status
  values (`Proposed`/`Ready`/`In Progress`/`Done`/`Blocked`) from the lifecycle
  narrative vocabulary used in index commentary, names the parser's
  `Draft → Proposed` / `Complete → Done` normalisations, marks the release
  metadata block as a prose convention, and documents `Test:` as a legacy alias
  for `Validation:`. `packages/aps/README.md` replaces links to nonexistent
  `docs/` files with the canonical spec URL plus pointers to `AGENTS.md`,
  `examples/`, and `templates/`. `packages/aps/AGENTS.md` lists the validator's
  actual rule set (15 rule names emitted by `validator/index.ts`, not 8),
  documents the leaf vs index document shapes, and tables the parser tolerances
  for field aliases and status normalisations.
- **Confidence:** medium

### DOCGOV-004: Repair ADR integrity and enforcement

- **Status:** Complete
- **Intent:** Make ADR numbering, lifecycle, and decision-log coverage
  mechanically trustworthy.
- **Expected Outcome:** Duplicate/missing ADR index entries are resolved, the ADR
  process guide matches the current repository, and a validation path exists for
  future ADR changes.
- **Validation:** `pnpm test:adr-integrity && pnpm adr:check && pnpm format:check`
- **Dependencies:** DOCGOV-002
- **Files:** `plans/decisions/DECISION-LOG.md`,
  `plans/decisions/021-in-house-nx-rust-plugin.md` (renamed from
  `026-in-house-nx-rust-plugin.md`), `plans/archive/modules/nx-rust-plugin.aps.md`,
  `plans/archive/modules/rust-nx-migration.aps.md`,
  `docs/guides/adr-process.md`, `scripts/docs/adr-integrity.sh`,
  `scripts/docs/adr-integrity.test.sh`, `package.json`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Closeout:** Validation passed with
  `pnpm test:adr-integrity && pnpm adr:check && pnpm format:check` ahead of
  merge. ADR-026 duplicate resolved by renaming the in-house nx-rust
  plugin ADR to ADR-021 (fills the previously empty 021 slot); cross-references
  in the two archived nx-rust modules updated with a renumber note.
  `DECISION-LOG.md` rebuilt with all 42 ADR files indexed (added previously
  unindexed ADR-025, -032, -036, -037, -038, -039, -040), section ordering
  by number within section, and a new "Edge and Infrastructure" section
  for ADR-032. `docs/guides/adr-process.md` updated to point at
  `pnpm test:adr-integrity`, codify the no-gap / no-duplicate invariants,
  and require a DECISION-LOG row in the same PR. New script
  `scripts/docs/adr-integrity.sh` (plus fixture tests in
  `adr-integrity.test.sh`) checks for duplicate numbers, log/file orphans,
  and prints the next available ADR number; wired in as
  `pnpm adr:check` (one-shot) and `pnpm test:adr-integrity` (fixture tests).
- **Confidence:** high

### DOCGOV-005: Add documentation validation baseline

- **Status:** Complete
- **Intent:** Convert closeout from memory-based hygiene into fully automated
  checks.
- **Expected Outcome:** `pnpm docs:check` validates metadata, tags, links,
  APS/index consistency, ADR integrity, generated-index freshness, and as-built
  source path existence. Manual indexing is not allowed; the only manual input is
  document-local metadata and approved tag catalogue updates. Until this ships,
  references to `pnpm docs:check` are target-state guidance, not an available
  repository command.
- **Validation:** `pnpm docs:check && pnpm test:docs-check && pnpm format:check && pnpm lint:check`
- **Dependencies:** DOCGOV-002, DOCGOV-004
- **Files:** `plans/decisions/042-closeout-enforcement-exit-codes.md`,
  `plans/execution/DOCGOV-005.steps.md`, `packages/docs-meta/**`,
  `scripts/docs/{docs-check.mjs,check-metadata.mjs,check-tags.mjs,check-links.mjs,check-aps.mjs,check-adr.mjs,check-index-freshness.mjs,check-asbuilt-paths.mjs,docs-check.test.sh}`,
  `docs/governance/{tags-catalogue.md,docs-check.baseline.json}`,
  `package.json`, `.github/workflows/ci.yml`,
  `pnpm-workspace.yaml`, `tsconfig.base.json`,
  `plans/decisions/DECISION-LOG.md`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Closeout:** Validation passed with
  `pnpm docs:check && pnpm test:docs-check && pnpm format:check && pnpm lint:check`
  ahead of merge. A planning council (session `plan-0b3290b4`) settled the
  design across nine decisions before any code landed; the outcome is
  recorded in [ADR-042](../decisions/042-closeout-enforcement-exit-codes.md)
  (closeout-enforcement carve-out from ADR-002) and the 24-step action plan
  at [`../execution/DOCGOV-005.steps.md`](../execution/DOCGOV-005.steps.md).
  Ships `pnpm docs:check` as a thin Node ESM orchestrator over seven
  surfaces (`metadata`, `tags`, `links`, `aps`, `adr`, `index-freshness`,
  `asbuilt-paths`) — five real validators plus two no-op stubs reserved for
  DOCGOV-006 and DOCGOV-007. The metadata parser lives in a new
  `@eddacraft/anvil-docs-meta` package (mirrors `packages/aps` Zod-validated
  patterns) so DOCGOV-006 / DOCGOV-007 can reuse it. The tags surface reads
  the seeded `docs/governance/tags-catalogue.md` (audited from current
  `Tags:` usage across 195 APS files); the links surface resolves files and
  GitHub-style heading anchors across `docs/**` and `plans/**`; the APS and
  ADR surfaces wrap `pnpm aps:drift` and `pnpm adr:check` so existing logic
  is reused, not duplicated. The validator applies the ADR-003 new-edges-only
  discipline via `docs/governance/docs-check.baseline.json` (562 current
  errors absorbed; DOCGOV-008 will shrink the baseline as cleanup lands),
  and the orchestrator emits a labelled summary so a single CI failure
  points at the specific surface that broke. Wired into the `Docs Lint`
  GitHub Actions job; fixture tests cover all nine surface/baseline
  contract cases.
- **Confidence:** medium

### DOCGOV-006: Standardise runbook and as-built freshness

- **Status:** Complete
- **Authorisation:** Operator authorised execution on 2026-05-22 via "start
  executing what remains of DOCGOV" while the item was Proposed; dependencies
  DOCGOV-002 and DOCGOV-005 were already complete.
- **Intent:** Give runbooks and as-built docs a freshness contract — owner,
  scope, verification date, source path/tag references, and stale-state
  signals — and convert the `asbuilt-paths` stub shipped by DOCGOV-005 into a
  real validator that checks every cited source path resolves at the
  document's stated tag/SHA.
- **Expected Outcome:** `docs/architecture/_as-built-template.md` and a sibling
  runbook template define the freshness metadata block; the
  `@eddacraft/anvil-docs-meta` schema accepts the new fields without breaking
  existing docs; `scripts/docs/check-asbuilt-paths.mjs` resolves source
  references (file existence at the cited revision, surfacing missing or moved
  paths as new-edges-only findings against
  `docs/governance/docs-check.baseline.json`); representative as-built and
  runbook docs are migrated as worked examples. Stub log line in
  `check-asbuilt-paths.mjs` is removed.
- **Validation:** `pnpm docs:check && pnpm test:docs-check && pnpm format:check`
- **Dependencies:** DOCGOV-002, DOCGOV-005
- **Files:** `docs/architecture/_as-built-template.md`,
  `plans/execution/DOCGOV-006.steps.md`,
  `docs/guides/runbook-template.md` (new),
  `docs/guides/documentation-governance.md`,
  `docs/runbooks/release-runbook.md`,
  `docs/guides/release-doc-checklist.md`,
  `docs/guides/anvil-rule-authoring.md`,
  `docs/architecture/rust-architecture-endstate.md`,
  `packages/docs-meta/**`,
  `scripts/docs/check-asbuilt-paths.mjs`,
  `scripts/docs/docs-check.test.sh`,
  `docs/governance/docs-check.baseline.json`,
  `plans/modules/documentation-governance.aps.md`,
  `plans/modules/documentation-sync.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-009 (backfill applies the new runbook /
  as-built freshness fields onto legacy live docs), DOCGOV-005 (replaces the
  asbuilt-paths surface stub)
- **Absorbed from DOCSYNC** (2026-05-22 scope sharpening — these were filed
  against DOCSYNC but target internal runbook/architecture docs, not the
  public docs-site, and so naturally land under DOCGOV-006's freshness
  contract). The specific cleanup items must pass before DOCGOV-006 closes:
    - *(ex-DOCSYNC-015)* Gate-runner runbook section — CPU/latency envelope
      and `registry.json` resolution failure recovery
    - *(ex-DOCSYNC-017)* Name `pnpm test:scanner-parity` as a named preflight
      gate in `docs/runbooks/release-runbook.md`
    - *(ex-DOCSYNC-018)* Document rayon pool scope / `RAYON_NUM_THREADS`
      behaviour for `anvil-checks` in `rust-architecture-endstate.md`
    - *(ex-DOCSYNC-019)* Extend `docs/guides/release-doc-checklist.md` to
      include `anvil-rule-authoring.md`, `integrations/vscode.md`, and
      `integrations/mcp.md` for release doc sync
    - *(ex-DOCSYNC-020)* Add ReDoS-risk framing for RL-* rule authors
      (untrusted PR body / commit-message inputs) in
      `docs/guides/anvil-rule-authoring.md`
- **Closeout:** Validation passed with
  `pnpm docs:check && pnpm test:docs-check && pnpm format:check` after
  regenerating `docs/governance/docs-check.baseline.json`; parser coverage also
  passed with `pnpm -F @eddacraft/anvil-docs-meta test`. The metadata parser now
  exposes parsed freshness anchors and source references; `check-asbuilt-paths.mjs`
  validates governed as-built/runbook source paths and participates in baseline
  regeneration. Added `docs/guides/runbook-template.md`, refreshed the as-built
  template, and closed the absorbed DOCSYNC-015/-017/-018/-019/-020 notes in the
  touched docs.
- **Confidence:** medium

### DOCGOV-007: Generate or reconcile documentation indexes

- **Status:** Complete
- **Authorisation:** Operator authorised execution on 2026-05-22 via "open PR
  and move onto next" while the item was Proposed; dependency DOCGOV-005 is
  complete. Work originally started from `main` to keep generated-index work
  isolated, then rebased after DOCGOV-006 landed.
- **Intent:** Replace hand-maintained documentation discovery with generated
  indexes driven by document metadata, and turn the `index-freshness` stub
  shipped by DOCGOV-005 into a real validator.
- **Expected Outcome:** `pnpm docs:index` generates indexes by type, authority,
  owner, status, and tag from document metadata into a known set of generated
  files (header marker identifies them as generated); `pnpm docs:index:check`
  fails when those generated indexes are stale relative to current metadata,
  and `scripts/docs/check-index-freshness.mjs` invokes the same logic so a
  single `pnpm docs:check` run catches drift. New tags are added through the
  approved tag catalogue (`docs/governance/tags-catalogue.md`), not by
  manually editing indexes. Stub log line in `check-index-freshness.mjs` is
  removed.
- **Validation:** `pnpm docs:index:check && pnpm docs:check && pnpm test:docs-check && pnpm format:check && pnpm lint:check`
- **Dependencies:** DOCGOV-005
- **Files:** `scripts/docs/docs-index.mjs` (new),
  `plans/execution/DOCGOV-007.steps.md`,
  `scripts/docs/check-index-freshness.mjs`,
  `scripts/docs/docs-check.test.sh`,
  `packages/docs-meta/**`,
  `docs/governance/tags-catalogue.md`,
  `docs/**` (generated index files under `docs/indexes/` or equivalent),
  `package.json`, `.github/workflows/ci.yml`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-009 (metadata backfill is the input
  generated indexes read from — generation quality depends on backfill
  coverage), DOCGOV-010 (reorg changes paths the generator must traverse),
  DOCGOV-005 (replaces the index-freshness surface stub)
- **Closeout:** Validation passed with
  `pnpm docs:index:check && pnpm docs:check && pnpm test:docs-check && pnpm format:check && pnpm lint:check`
  on branch `docs/docgov-indexes`. `scripts/docs/docs-index.mjs` now scans
  governed Markdown metadata, writes formatter-stable generated indexes under
  `docs/indexes/`, and supports `--check` / `--json` for CI and
  `docs:check` integration. Generated indexes cover type, authority, owner,
  status, and approved tag usage where available. `check-index-freshness.mjs`
  delegates to the generator in check mode, removing the DOCGOV-007 stub while
  preserving DOCGOV-006's real `asbuilt-paths` validator after rebase.
- **Confidence:** medium

### DOCGOV-008: Migrate stale entrypoints and archive dead docs

- **Status:** Complete
- **Execution plan:** [`plans/execution/DOCGOV-008.steps.md`](../execution/DOCGOV-008.steps.md); audit evidence + closeout delta at [`plans/execution/DOCGOV-008.audit.md`](../execution/DOCGOV-008.audit.md). All 6 tasks completed on `docs/docgov-008-archive-dead-docs` (release-runbook relocation via Option A; 19 archive moves; 20 files moved out-of-repo to `~/Projects/anvil-gtm-wip/`; entrypoint relinks; baseline `metadata` shrank 179 → 140). PR [#1892](https://github.com/eddacraft/anvil-001/pull/1892) closed after the branch reached `main` at `14a40a78`; fresh closeout validation passed on 2026-05-24.
- **Intent:** Reduce ambiguity by fixing stale onboarding links, archiving
  dead operational docs out of active paths, and resolving the long-standing
  release-runbook migration exception — done before the live-doc backfill /
  reorg work so DOCGOV-009 and DOCGOV-010 don't waste effort on docs that
  are about to leave.
- **Expected Outcome:** Contributor entrypoints (`README.md`, `AGENTS.md`,
  `CLAUDE.md`, `docs/README.md`, package READMEs) route through current /
  generated indexes; clearly-dead specs and guides are archived via
  `git mv` to `docs/archive/**` (or `plans/archive/**` where APS-linked) with
  redirect stubs only where inbound links exist outside the repo; the
  legacy `docs/guides/release-runbook.md` migration exception is closed by
  relocating the runbook to `docs/runbooks/release-runbook.md` (Option A —
  locked 2026-05-23 in `plans/execution/DOCGOV-008.steps.md`);
  public-vs-internal docs platform claims in
  `docs/README.md` and `docs/guides/documentation-governance.md` are
  reconciled against current reality.
- **Validation:** `pnpm docs:check && pnpm format:check`
- **Dependencies:** DOCGOV-005, DOCGOV-007
- **Files:** `README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/README.md`,
  `docs/runbooks/release-runbook.md`,
  `docs/guides/documentation-governance.md`, `docs/archive/**` (new),
  package and crate README cross-references,
  `docs/governance/docs-check.baseline.json`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-009 (run archive first so backfill skips
  dead docs), DOCGOV-010 (archive moves complete before live reorg so the
  same doc isn't shuffled twice)
- **Closeout:** Validation passed with
  `pnpm docs:check && pnpm docs:index:check && pnpm format:check && pnpm lint:check && pnpm test:docs-check`
  on 2026-05-24. `pnpm docs:check` reported 7/7 surfaces passing under the
  refreshed baseline; `pnpm docs:index:check` reported 0 errors / 0 warnings;
  format, lint, Rust clippy/fmt-check, and docs-check fixture tests passed. One
  pre-existing ESLint warning remains in
  `crates/anvil-checks-napi/index.d.ts` as non-blocking lint output.
- **Confidence:** medium

### DOCGOV-009: Backfill metadata on existing live documentation

- **Status:** Merged
- **Authorisation:** Promoted from Proposed on 2026-05-25 after DOCGOV-008
  completed and the operator requested DOCGOV-009 Task 1. Metadata backfill is
  authorised after rubric sign-off on 2026-05-25; default owner rules are
  accepted, with `@aneki` (`aneki@eddacraft.ai`) as backup/admin owner. Merged
  2026-05-25 via PR [#1927](https://github.com/eddacraft/anvil-001/pull/1927)
  at `02fc92f5`.
- **Intent:** Apply the DOCGOV-002 taxonomy and metadata convention to live
  documents that predate it, so the entire active doc set declares type,
  authority, owner, status, freshness, and upstream/downstream references on
  the same contract.
- **Expected Outcome:** Live docs under `docs/**` (and any other governed
  paths) carry the canonical metadata block; the
  `docs/governance/docs-check.baseline.json` baseline shrinks as backfills
  land; tags resolve against the approved catalogue rather than ad-hoc usage.
  Dead docs identified during backfill are routed to DOCGOV-008 archival review
  rather than rewritten in place.
- **Scope note:** The Merged #1927 increment landed the first high-authority
  batch only; the remaining live-doc backfill waves continue under DOCGOV-011 so
  this item's Merged record stays anchored to what #1927 actually shipped.
- **Validation:** `pnpm docs:check && pnpm docs:index:check && pnpm format:check`
- **Dependencies:** DOCGOV-002, DOCGOV-005, DOCGOV-008
- **Files:** `docs/**`, `docs/governance/docs-check.baseline.json`,
  `docs/governance/tags-catalogue.md`,
  `plans/execution/DOCGOV-009.steps.md`,
  `plans/execution/DOCGOV-009.rubric.md`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-008 (route dead docs to archive instead of
  backfilling), DOCGOV-010 (provides the authority/type tags that drive
  reorganisation placement), DOCGOV-011 (continues the remaining backfill waves)
- **Confidence:** medium

### DOCGOV-010: Reorganise live documentation under canonical taxonomy

- **Status:** Proposed
- **Intent:** Move existing live documents into a coherent folder structure
  driven by the DOCGOV-002 taxonomy (type, authority, owner) rather than
  today's mixed historical layout, now that the validation baseline and
  metadata backfill make placement decisions mechanical instead of judgement
  calls.
- **Expected Outcome:** Live docs sit under taxonomy-aligned paths; inbound
  links from `AGENTS.md`, `CLAUDE.md`, package READMEs, indexes, and ADRs are
  updated in the same PR (or via redirect stubs) so `pnpm docs:check` link
  validation stays green; the reorganised layout is documented in
  `docs/guides/documentation-governance.md` so future docs land in the right
  place by default.
- **Validation:** `pnpm docs:check && pnpm format:check`
- **Dependencies:** DOCGOV-005, DOCGOV-008, DOCGOV-009
- **Files:** `docs/**`, `docs/guides/documentation-governance.md`,
  `docs/governance/docs-check.baseline.json`, `AGENTS.md`, `CLAUDE.md`,
  package and crate README cross-references,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-007 (generated indexes consume the new
  layout), DOCGOV-008 (archive moves run before live reorg to avoid
  shuffling docs that are about to leave), DOCGOV-009 (metadata backfill
  provides the authority/type tags placement depends on)
- **Confidence:** medium

### DOCGOV-011: Continue metadata backfill across remaining live docs

- **Status:** In Progress
- **Authorisation:** Continuation of DOCGOV-009 under the same operator-approved
  rubric (`plans/execution/DOCGOV-009.rubric.md`, signed off 2026-05-25).
  DOCGOV-009 Merged via #1927 landed the first high-authority batch; this item
  carries the remaining backfill waves. Authorised 2026-05-27.
- **Intent:** Apply the DOCGOV-002 metadata table to the live docs DOCGOV-009 did
  not reach, so the governed doc set converges on a single metadata contract.
- **Expected Outcome:** Architecture, runbook, vision, observability, strategy,
  testing, internal, and guide docs that lack the governance metadata table
  carry it; the `docs/governance/docs-check.baseline.json` `metadata` bucket
  shrinks by each backfilled wave with no new findings introduced. Docs not yet
  reached stay tracked in the baseline for later waves until the bucket is empty.
- **Validation:** `pnpm docs:check && pnpm docs:index:check && pnpm format:check`
- **Dependencies:** DOCGOV-002, DOCGOV-005, DOCGOV-009
- **Files:** `docs/**`, `docs/governance/docs-check.baseline.json`,
  `plans/modules/documentation-governance.aps.md`, `plans/index.aps.md`
- **Coordinates with:** DOCGOV-009 (same rubric and metadata contract),
  DOCGOV-010 (metadata authority/type tags drive reorganisation placement)
- **Confidence:** high

### DOCGOV-012: Harden the docs-check gating tooling against malformed input and flag misrouting

- **Status:** Draft
- **Authorisation:** Not yet authorised for implementation — filed as a tracked
  backlog item from the 2026-05-29 clawpatch periodic scan. Needs the normal
  operator/APS go-ahead (or a hotfix decision) before code work begins.
- **Intent:** Make the DOCGOV-owned `scripts/docs` validation surfaces fail
  safely: a malformed link or a partial surface failure should produce a labelled
  finding or abort cleanly, never silently lose baseline data or crash the
  orchestrator.
- **Expected Outcome:** Three confirmed/contract defects are fixed:
  1. `--update-baseline` (`docs-check.mjs` `regenerateBaseline`) preserves or
     reloads existing baseline entries for any surface that failed to produce
     valid JSON, skips the final write unless regeneration fully succeeded, and
     exits non-zero when a baselineable surface fails — so a partial failure can
     no longer overwrite `docs/governance/docs-check.baseline.json` with dropped
     entries.
  2. `docs-check.mjs` only forwards baseline flags (e.g. `--no-baseline`) to
     surfaces where `surface.baselineable` is true, so non-baselineable surfaces
     like `check-index-freshness` / `docs-index.mjs` are no longer handed a flag
     they reject.
  3. `check-links.mjs` `resolveLink` wraps percent-decoding so a malformed
     percent escape yields a labelled ERROR finding for the link/file/line rather
     than an uncaught `URIError`.
- **Validation:** Fixture tests for each defect — a forced invalid-JSON surface
  that asserts `--update-baseline` fails without dropping an existing entry; an
  orchestrator `--no-baseline` run that does not fail index-freshness on an
  unknown option; a `check-links` malformed-percent-escape fixture that asserts a
  labelled ERROR and non-zero exit. Then
  `pnpm docs:check && pnpm docs:index:check && pnpm format:check`.
- **Identified From:** 2026-05-29 clawpatch periodic scan
  (`plans/audits/2026-05-29-clawpatch-periodic-scan.json`, findings
  `fnd_sig-feat-library-34bc4660c0-5f3f` /
  `fnd_sig-feat-library-34bc4660c0-d3a8` /
  `fnd_sig-feat-library-34bc4660c0-2aa7`); triage at
  `plans/reviews/2026-05-29-clawpatch-triage.md`. Defect 1 corroborates the
  hand-noted `--update-baseline` overwrite friction in
  `plans/reviews/continuous-improvement-log.md`, which was never filed.
- **Dependencies:** DOCGOV-005 (built `docs:check`), DOCGOV-007 (index-freshness
  surface)
- **Files:** `scripts/docs/docs-check.mjs`, `scripts/docs/check-links.mjs`,
  `docs/governance/docs-check.baseline.json` (only if regeneration semantics
  change require it), plus fixture tests under the docs-check test surface.
- **Coordinates with:** DOCGOV-006/-007 (same surface family); GH tracking issue
  pending operator confirmation.
- **Confidence:** high
