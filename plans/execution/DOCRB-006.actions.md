# DOCRB-006 Central Architecture Views Action Plan

**Work item:** DOCRB-006
**Status:** In Progress
**Risk:** high — the diagrams describe trust, deployment, validation, and failure boundaries
**Base:** `d9b30b23daef0da05f74a7d44dfa3accd0e03fe7`

## Goal

Establish the small authoritative central architecture set defined by ADR-123
and the DOCRB corpus disposition, retire duplicate or obsolete central diagram
authorities, and leave source-traceable Mermaid views that do not repeat local
component internals.

## Authority and boundaries

- `docs/architecture/overview.md` owns only system context and
  container/component relationships.
- `docs/architecture/trust-and-deployment-boundaries.md` owns the macro trust
  and deployment view and links to detailed BAUTH, INTD, APGOV, and docs-shell
  authorities.
- `docs/architecture/save-to-validation.md` owns the cross-owner save flow and
  keeps caller-buffer `scan_buffer` modes separate from post-save
  `validate_paths`.
- `docs/architecture/docs-delivery.md` owns the macro source/build/deployment
  and shell/private/public renderer flow while retaining the DOCRB/DSITE owner
  gap.
- KERN-owned `docs/architecture/quality-model.md`, BAUTH-owned
  `auth-as-built.md`, and EDDA-owned `edda-stack.md` retain their detailed
  central concerns. The Rust overview and
  adapter workflow remain linked authorities.
- Component-internal migration is DOCRB-005. Public Draw.io/SVG assets are
  DOCRB-007/-008. Automated Mermaid tooling and mandatory change
  classification are DOCRB-009.

## File map

- `plans/modules/docs-rebaseline.aps.md` — exact item contract and lifecycle.
- `plans/index.aps.md` — current NBI wording only.
- `plans/execution/DOCRB-006.actions.md` — this execution sequence.
- `plans/specs/2026-08-17-docrb-corpus-disposition.md` — final diagram dispositions.
- `CONTEXT.md` — central architecture discovery pointers.
- `docs/guides/documentation-governance.md` — disposition-alignment and
  freshness review.
- `docs/reviews/shipped-codebase-review-checklist.md` — remove stale overview
  layering/policy pointers.
- `docs/architecture/README.md` — view discovery and retired Draw.io links.
- `docs/architecture/overview.md` — system-context and container/component views.
- `docs/architecture/quality-model.md` — remove authority-cycle metadata and retain the quality view.
- `docs/architecture/auth-as-built.md` — retain the detailed BAUTH view in renderable form.
- `docs/architecture/edda-stack.md` — retain the EDDA-owned promotion view in renderable form.
- `docs/architecture/trust-and-deployment-boundaries.md` — new macro trust view.
- `docs/architecture/save-to-validation.md` — new cross-owner validation sequence.
- `docs/architecture/docs-delivery.md` — new production docs-delivery view.
- `docs/runbooks/save-time-background-driver.md` — link the operational path to the central sequence.
- `docs/architecture/anvil-system-components.drawio` — retire after replacement.
- `docs/architecture/pptx-workflow.drawio` — retire as obsolete.
- `docs/indexes/by-authority.md`, `by-owner.md`, `by-status.md`, and
  `by-type.md` — generated discovery outputs if metadata changes require them.
- `plans/reviews/2026-08-20-docrb-006-central-views.md` — source-edge, render,
  link, duplication, retirement, and validation evidence.

## Actions

### 1. Lock source truth and disposition

- Reconcile the five required views against current source and local pilot docs.
- Record audience, concern, owner, lifecycle, upstreams, local-authority
  relationship, and adjacent textual meaning for every retained view.
- Update the corpus disposition rather than creating another inventory.
- Treat archived references to retired Draw.io files as history, not live links.

**Checkpoint:** each intended node, edge, boundary, fallback, and retirement has
a current source anchor and one owner.

### 2. Rebuild the central set without duplicate authority

- Reduce `overview.md` to system context and container/component relationships.
- Add the trust/deployment, save-to-validation, and docs-delivery views.
- Remove overview duplicates of quality, generic check-pipeline, and EDDA
  internals; link their owning documents instead.
- Keep the macro trust view subordinate to detailed BAUTH/INTD/APGOV/local
  authorities.
- Delete the two retired Draw.io sources after their live links and disposition
  are updated.

**Checkpoint:** the five required DOCRB-006 views are discoverable alongside
retained supporting central authorities, and no concern has two apparent
authorities.

### 3. Prove renderability and source accuracy

- Extract every changed Mermaid block from its owning Markdown file.
- Render with temporary `@mermaid-js/mermaid-cli@11.16.0` tooling; use a
  temporary Puppeteer `--no-sandbox` configuration only if Chromium cannot
  start its nested sandbox.
- Keep all inputs and SVG outputs under `/tmp`; record non-empty byte counts.
- Manually trace every material arrow and boundary to the adjacent cited source.
- Resolve every repository-local Markdown link in changed documentation.
- Prove retired Draw.io paths have no live inbound references while preserving
  historical archive references.

**Checkpoint:** all changed Mermaid blocks render, all links resolve, and the
evidence report contains a per-view source-edge trace.

### 4. Validate and review

Run:

```text
pnpm format:check
pnpm docs:index
pnpm docs:index:check
pnpm docs:check
pnpm aps:active-lint
pnpm aps:index:check
pnpm aps:drift --json
git diff --check
```

Then run independent `verify-loop` over the exact base/head and Council review
with explicit trust/deployment and source-accuracy scrutiny.

**Checkpoint:** repository gates pass, inherited sibling advisories are
distinguished from DOCRB findings, independent verification passes, and Council
has no unresolved blocking finding.

## Rollback

Revert the DOCRB-006 commits as one documentation-only unit. No runtime,
deployment, public asset, or mandatory CI behaviour is changed.
