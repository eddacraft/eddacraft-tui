# DOCRB-008 — Public information architecture and curated diagrams

## ReadyItem

- **Target:** DOCRB-008
- **Decision:** ready
- **Goal:** Make the mounted public documentation easier to follow without changing product content authority.
- **Expected behaviour:**
  - Production sidebars name tutorial, how-to, reference, and explanation placement explicitly.
  - Exactly two high-value public journeys have authoritative paired Draw.io/SVG diagrams with accessible owning-page references.
  - Existing routes, page ownership, copied-content attestations, and release/version provenance remain unchanged.
  - The private, public, and shell production documentation builds succeed.
- **Dependencies:** DOCRB-006, DOCRB-007, and DOCRB-011 are Merged on `origin/main` at `7809a06801c583a740dda4abc18828d3a4970fdb`.
- **Scope:** Four label/grouping-only production sidebars; `docs/public/anvil/assets/diagrams/detect-fix-verify.{drawio,svg}` owned by `docs/public/anvil/first-gate.md`; `docs/public/aps/assets/diagrams/work-item-lifecycle.{drawio,svg}` owned by the already-mounted `docs/public/aps/getting-started.md`, which links onward to workflow authority; the public-diagram inventory row/notes in `plans/specs/2026-08-17-docrb-corpus-disposition.md`; the exact approved two-file prerequisite in `scripts/docs/lib/public-diagrams.mjs` and `scripts/docs/check-public-diagrams.test.mjs`; the seven gating and one baselined direct file-level freshness closeouts in `docs/guides/architecture-diagrams.md`, `docs/guides/documentation-governance.md`, `docs/guides/README.md`, `docs/architecture/README.md`, `docs/governance/tags-catalogue.md`, `docs/guides/adapters/README.md`, `docs/reviews/README.md`, and `docs/README.md`; the current-item lifecycle reconciliation in `plans/index.aps.md`; this action plan; and one evidence report at `plans/reviews/2026-08-21-docrb-008-public-ia.md`. The final approved scope is 25 paths.
- **Approved prerequisite:** On 2026-08-21 the operator directly approved exact, anchored removal of Draw.io Desktop 31.1.8's three-line official SVG prolog before hashing and annotation, plus fail-closed regression coverage. The approval is limited to the two named script paths.
- **Non-scope:** Content moves or rewrites, `docs/public/aps/workflow.md`, APS sidebar changes, generated references, extra diagrams, tooling beyond the exact approved prerequisite, dependencies or lockfiles, checker weakening, `apps/docs-site/**`, PR #4050 absorption, sibling-module status, release publishing, and DOCRB-009/-010.
- **Validation:** `node --test scripts/docs/check-public-diagrams.test.mjs && pnpm docs:public:check && pnpm docs:public:diagrams && pnpm docs:check && pnpm docs:owed && pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm --filter @eddacraft/docs-shell build`
- **Readiness compatibility:** Installed `aps` treats the project's valid `Merged <date> via PR #N` extension as `Unknown`; `aps start DOCRB-008` therefore reports DOCRB-006, DOCRB-007, and DOCRB-011 as unmet. Exact source rows and merge receipts were independently verified at the pinned base, so the authorised manual transition is limited to DOCRB-008.
- **Export constraint:** Only authentic Draw.io Desktop 31.1.8 may produce the SVGs. No hand-authored SVG, provenance weakening, package installation, repository dependency, or alternate version is permitted.

## Actions

### 1. Make public information types explicit

- **Checkpoint:** Four live sidebars expose explicit information-type labels
- **Validate:** `pnpm docs:public:check`

### 2. Repair the exact official-prolog exporter seam

- **Checkpoint:** The exporter normalises only Draw.io Desktop 31.1.8's exact anchored prolog before annotation and hashing; altered doctypes, entity/system doctypes, other processing instructions, and hash tampering still fail closed.
- **Validate:** `node --test scripts/docs/check-public-diagrams.test.mjs`

### 3. Add the two curated public journey diagrams

- **Depends on:** 1, 2
- **Checkpoint:** Two accessible Draw.io/SVG pairs have one owner each and the corpus inventory reflects them.
- **Validate:** `pnpm docs:public:diagrams && pnpm docs:check`

### 4. Close the eight direct file-level freshness edges

- **Depends on:** 2, 3
- **Checkpoint:** Diagram procedure names the exact normalisation; governance metadata records the compatible inventory review; and the AICON-, DOCRB-, and DOCGOV-owned direct consumers record compatible discovery, catalogue, and placement reviews.
- **Validate:** `pnpm docs:owed && pnpm docs:check`

### 5. Record source, navigation, and build evidence

- **Depends on:** 1, 2, 3, 4
- **Checkpoint:** Evidence traces diagram meaning, official export provenance, navigation, and preserved authority.
- **Validate:** `pnpm --filter @eddacraft/anvil-docs-private build && pnpm --filter @eddacraft/docs-public build && pnpm --filter @eddacraft/docs-shell build`

### 6. Close the bounded Council findings

- **Depends on:** 1, 2, 3, 4, 5
- **Checkpoint:** DOCRB-008 lifecycle truth is consistent without changing the stored 8/11 count; both existing diagrams have an opaque accessible canvas exported by authentic Draw.io Desktop 31.1.8 and remain legible on mounted light and dark pages; and freshness provenance uses durable APS/evidence anchors rather than branch commit IDs.
- **Validate:** `pnpm docs:public:diagrams && pnpm docs:owed && pnpm docs:check && pnpm aps:active-lint && pnpm aps:index:check`
