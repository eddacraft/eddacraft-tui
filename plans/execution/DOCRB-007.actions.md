# DOCRB-007 Public Draw.io-to-SVG Pipeline Action Plan

**Work item:** DOCRB-007
**Status:** In Progress
**Risk:** high — mandatory documentation tooling processes attacker-controlled XML/SVG and filesystem paths
**Base:** `e23a60200093dab330b5d61c92a0ae0fdc2a9d85`

## ReadyItem

- **Goal:** Establish a reproducible, source-controlled Draw.io Desktop to accessible SVG pipeline for mounted public documentation families.
- **Work item:** DOCRB-007
- **Status:** Ready
- **Expected behaviour:** Mounted public family roots accept only lower-kebab paired `.drawio`/`.svg` assets; the pinned exporter embeds canonically matching source, writes atomically without following symlinks, and records deterministic provenance; namespace-aware checks reject stale, active, inaccessible, mount-drifted, weakly referenced, or unreviewed raster diagram assets; both production Docusaurus renderers build.
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

Create a single machine-readable contract for the five mounted family roots,
their production renderer configs, Draw.io Desktop version, and exact export
flags. The checker must also prove those declared roots remain mounted.

**Checkpoint:** an unmounted or undeclared family fails while the disabled
`start-here` family and rollback-only `docs-site` do not enter scope.

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
provenance contract, accessibility/reference requirements, and exclusions in
the existing diagram authority guide.

**Checkpoint:** `test:docs-check` proves the twelfth surface and
`docs:check` passes the live corpus with zero governed diagrams.

### 5. Prove builds and close out evidence

Build both production renderers, refresh generated documentation indexes only
if required, run exact-range documentation/APS/diff gates, and record command
exits, fixture behaviours, mounted-family evidence, scope exclusions, and
changed paths in the review report.

**Checkpoint:** the report-inclusive commit candidate passes every ReadyItem
command and retains no production diagram asset.

## Council FIX ALL repair

The exact-head Council review at `e1eebcaa8` raised the implementation risk.
The repair keeps the original file and publication boundary while adding these
binding behaviours:

1. fail-closed SVG/XML token, namespace, entity, CSS, and reference inspection;
2. canonical embedded-source equality and stronger provenance;
3. canonical non-symlink family confinement plus atomic destination replacement;
4. structural exact-set mount extraction from both production renderer ASTs;
5. Markdown AST references with meaningful alt text or an auditable
   target-bound description marker;
6. candidate-scoped raster enforcement with an ADR-123 reviewed exception; and
7. exact semver stdout equality with the actual output recorded, while the
   operator remains responsible for selecting an authentic Desktop binary.

Each behaviour has an adversarial replacement RED/GREEN fixture. No production
diagram, renderer mount, dependency, lockfile, CI workflow, or excluded surface
enters scope.

## Rollback

Revert the DOCRB-007 commits as one documentation-tooling unit. With no
production diagrams authored by this work item, rollback removes only the new
checker/export contract, tests, guidance, and APS bookkeeping.
