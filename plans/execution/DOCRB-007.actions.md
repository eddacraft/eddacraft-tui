# DOCRB-007 Public Draw.io-to-SVG Pipeline Action Plan

**Work item:** DOCRB-007
**Status:** In Progress
**Risk:** high — mandatory documentation tooling processes attacker-controlled XML/SVG and filesystem paths
**Base:** `e23a60200093dab330b5d61c92a0ae0fdc2a9d85`

## ReadyItem

- **Goal:** Establish a reproducible, source-controlled Draw.io Desktop to accessible SVG pipeline for mounted public documentation families.
- **Work item:** DOCRB-007
- **Status:** Ready
- **Expected behaviour:** The five public-family `assets/diagrams` directories accept only lower-kebab paired `.drawio`/`.svg` assets; the pinned exporter embeds canonically matching source, writes atomically without following symlinks, and records deterministic provenance; namespace-aware checks reject malformed, stale, active, inaccessible, weakly referenced, or unreviewed raster diagram assets; both production Docusaurus renderers build.
- **Files:** `plans/modules/docs-rebaseline.aps.md`, `plans/index.aps.md`, `plans/execution/DOCRB-007.actions.md`, `plans/reviews/2026-08-20-docrb-007-public-svg-pipeline.md`, `package.json`, `scripts/docs/docs-check.mjs`, `scripts/docs/docs-check.test.sh`, `scripts/docs/check-public-diagrams.mjs`, `scripts/docs/export-public-diagram.mjs`, `scripts/docs/lib/public-diagrams.mjs`, `scripts/docs/public-diagrams.json`, `scripts/docs/check-public-diagrams.test.mjs`, `scripts/docs/fixtures/public-diagrams/**`, `docs/guides/architecture-diagrams.md`, and generated documentation indexes if freshness metadata changes require them.
- **Validation commands:** `node --test scripts/docs/check-public-diagrams.test.mjs`; `pnpm test:docs-check`; `pnpm docs:check`; `pnpm --filter @eddacraft/anvil-docs-private build`; `pnpm --filter @eddacraft/docs-public build`; `pnpm format:check`; `pnpm docs:index`; `pnpm docs:index:check`; `pnpm docs:owed --since e23a602000`; `pnpm aps:active-lint`; `pnpm aps:index:check`; `pnpm aps:drift --json`; `git diff --check`.
- **Dependencies:** DOCRB-001 and DOCRB-002 are Merged.
- **Risk:** high.
- **Design source:** ADR-123 and `plans/specs/2026-08-16-docs-rebaseline.md`.
- **Constraints / non-goals:** No production diagram authoring; no `docs/public/start-here` activation; no legacy `docs/architecture/**.drawio` handling; no DOCRB-009 component-Mermaid or affected-change enforcement.
- **PR base:** integration (`main`).
- **Stack depends on:** none.
- **Decision:** ready.

## Readiness evidence

At the exact base:

- `apps/anvil-docs-private/docusaurus.config.ts` mounts `docs/public/anvil` and `docs/public/beta`;
- `apps/docs-public/docusaurus.config.ts` mounts `docs/public/aps`, `docs/public/kindling`, and `docs/public/edda-stack`;
- `docs/public/start-here` is retained but disabled and is outside this pipeline;
- no live governed `.drawio` assets exist, so fixtures can establish the contract without authoring production diagrams;
- Draw.io Desktop `31.1.8` is the pinned exporter, using `--export --format svg --embed-diagram --crop --border 0`; and
- the existing docs, APS active, and drift gates run at the base; the inherited DOCDEF progress mismatch remains advisory and unrelated.

## Actions

### 1. Lock the governed surface

Create a single machine-readable contract for the five governed
`assets/diagrams` directories, their family roots, the Draw.io Desktop version,
and exact export flags.

**Checkpoint:** missing or duplicate directory declarations fail; files outside
the five explicit directories do not enter the governed asset set.

### 2. Build export and provenance behaviour vertically

Start with failing fixture assertions, then implement the export wrapper and
shared validation library. The wrapper verifies the pinned Desktop version,
exports a single-page SVG with embedded source, injects source/content hashes
and exporter provenance, and adds deterministic accessible title/description
metadata derived from the Draw.io source.

**Checkpoint:** a valid fixture passes; stale source, changed SVG, missing
embedded source, or mismatched version/flags fails.

### 3. Add safety, accessibility, naming, and reference checks

Add one failing case at a time for lower-kebab naming, sibling pairing,
raster-only assets, active content/external references, missing SVG accessible
name/description, and missing or empty-alt Markdown references.

**Checkpoint:** every negative fixture fails for its intended reason and the
governed valid fixture remains green.

### 4. Integrate the new docs surface and guidance

Add `public-diagrams` to the `docs:check` orchestrator and its labelled
surface contract tests. Document the pinned author workflow, supported paths,
provenance contract, accessibility/reference requirements, and scope boundary
in the existing diagram authority guide.

**Checkpoint:** `test:docs-check` proves the twelfth surface and
`docs:check` passes the live corpus with zero governed diagrams.

### 5. Prove builds and close out evidence

Build both production renderers, refresh generated documentation indexes only
if required, run exact-range documentation/APS/diff gates, and record command
exits, fixture behaviours, production-build evidence, scope boundary, and
changed paths in the review report.

**Checkpoint:** the report-inclusive commit candidate passes every ReadyItem
command and retains no production diagram asset.

## Council repair and operator-approved scope contraction

Council review correctly identified safety gaps in XML/SVG handling, path
confinement, accessibility references, raster scope, and exact exporter
provenance. Those repairs remain binding. Two later proposals exceeded the
DOCRB-007 acceptance boundary and the operator approved their removal:

- checker-time trusted Draw.io re-export and render attestation; and
- Docusaurus configuration AST analysis, exact mount-set enforcement, and
  exclusion-schema governance.

The retained contract covers structural sibling/embedded XML parity,
namespace-aware SVG safety, canonical non-symlink confinement, atomic output,
exact exporter version/output/arguments, source/export freshness hashes,
accessible real Markdown/MDX references, five explicit diagram directories,
directory-scoped ADR-123 raster exceptions, the twelfth `docs:check` surface,
fixture coverage, and both production renderer builds. Renderer integration is
proved at the system boundary by those builds, not by a new configuration
static-analysis subsystem.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. With no
production diagrams authored by this work item, rollback removes only the new
checker/export contract, tests, guidance, and APS bookkeeping.
