# DOCGOV-006 Implementation Plan

**Goal:** Replace the `asbuilt-paths` documentation-check stub with a real freshness/source-reference contract for runbooks and as-built docs.
**Architecture:** Keep metadata parsing in `@eddacraft/anvil-docs-meta` pure and filesystem-free, then perform repository path resolution in `scripts/docs/check-asbuilt-paths.mjs`. The validator remains new-edges-only through the existing docs-check baseline and only checks governed documents that declare `Type: As-built` or `Type: Runbook`.
**Tech Stack:** TypeScript/Zod metadata parser, Node ESM docs-check surface scripts, Markdown docs, APS.

---

## File Map

- `packages/docs-meta/src/types/index.ts` — add freshness/source-reference schemas and fields to parsed governance metadata.
- `packages/docs-meta/src/parser/parse-metadata.ts` — parse freshness anchors and code-wrapped source paths without filesystem access.
- `packages/docs-meta/src/parser/parse-metadata.test.ts` — pin the new parser contract.
- `scripts/docs/check-asbuilt-paths.mjs` — replace the DOCGOV-006 stub with a validator for governed as-built/runbook source references.
- `scripts/docs/docs-check.test.sh` — update contract tests now that `asbuilt-paths` is real.
- `docs/architecture/_as-built-template.md` — document the required freshness and source-reference shape.
- `docs/guides/runbook-template.md` — add the runbook freshness template.
- `docs/guides/documentation-governance.md` — describe source-reference validation and closeout expectations.
- `docs/guides/release-runbook.md` — add the scanner-parity preflight gate note absorbed from DOCSYNC.
- `docs/guides/release-doc-checklist.md` — expand release doc sync coverage.
- `docs/guides/anvil-rule-authoring.md` — add ReDoS-risk framing for PR-body and commit-message rules.
- `docs/architecture/rust-architecture-endstate.md` — document rayon pool / `RAYON_NUM_THREADS` behaviour for `anvil-checks`.
- `docs/governance/docs-check.baseline.json` — absorb current legacy-document findings after the new surface is active.
- `plans/modules/documentation-governance.aps.md` — record DOCGOV-006 execution state and closeout.
- `plans/modules/documentation-sync.aps.md` — keep the absorbed DOCSYNC follow-ups reconciled.
- `plans/index.aps.md` — keep DOCGOV progress/status consistent.

## Tasks

### Task 1: Parser Freshness Contract

**Files:**
- Modify: `packages/docs-meta/src/types/index.ts`
- Modify: `packages/docs-meta/src/parser/parse-metadata.ts`
- Test: `packages/docs-meta/src/parser/parse-metadata.test.ts`

- [ ] Write failing tests for parsed freshness anchors and source references.
- [ ] Run `pnpm -F @eddacraft/anvil-docs-meta test -- parse-metadata.test.ts` and verify failure.
- [ ] Add minimal schemas and parser extraction.
- [ ] Run `pnpm -F @eddacraft/anvil-docs-meta test -- parse-metadata.test.ts` and verify pass.

### Task 2: As-Built Paths Validator

**Files:**
- Modify: `scripts/docs/check-asbuilt-paths.mjs`
- Modify: `scripts/docs/docs-check.test.sh`

- [ ] Write failing shell-contract expectations for real `asbuilt-paths` output and JSON.
- [ ] Run `pnpm test:docs-check` and verify failure.
- [ ] Implement governed as-built/runbook path checking with baseline support.
- [ ] Run `pnpm test:docs-check` and verify pass.

### Task 3: Freshness Templates and Absorbed Docs

**Files:**
- Modify: `docs/architecture/_as-built-template.md`
- Create: `docs/guides/runbook-template.md`
- Modify: `docs/guides/documentation-governance.md`
- Modify: `docs/guides/release-runbook.md`
- Modify: `docs/guides/release-doc-checklist.md`
- Modify: `docs/guides/anvil-rule-authoring.md`
- Modify: `docs/architecture/rust-architecture-endstate.md`

- [ ] Add freshness/source-reference guidance to templates.
- [ ] Close the five absorbed DOCSYNC documentation notes.
- [ ] Run `pnpm docs:check` and inspect new findings.

### Task 4: Baseline and APS Closeout

**Files:**
- Modify: `docs/governance/docs-check.baseline.json`
- Modify: `plans/modules/documentation-governance.aps.md`
- Modify: `plans/modules/documentation-sync.aps.md`
- Modify: `plans/index.aps.md`

- [ ] Regenerate the docs-check baseline after the real surface lands.
- [ ] Mark DOCGOV-006 complete with validation evidence if all gates pass.
- [ ] Reconcile DOCSYNC absorbed follow-ups and index counts.
- [ ] Run `pnpm docs:check && pnpm test:docs-check && pnpm format:check`.
