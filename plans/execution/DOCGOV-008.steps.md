# DOCGOV-008 Implementation Plan

**Goal:** Reduce documentation ambiguity before the live-doc backfill by archiving clearly-dead docs, closing the `docs/guides/release-runbook.md` migration exception, routing contributor entrypoints through current/generated indexes, and reconciling public-vs-internal platform claims so DOCGOV-009 and DOCGOV-010 only handle live, classified material.
**Architecture:** Move-and-stub model. Dead docs go to `docs/archive/**` (or `plans/archive/**` for APS-linked artefacts) via `git mv` so history is preserved; redirect stubs only where inbound links from outside the repo exist. Entrypoints route through `docs/indexes/` and topical READMEs rather than re-listing surfaces inline. No source-of-truth content is duplicated; archived copies remain authoritative for history only.
**Tech Stack:** Markdown, `git mv`, `pnpm docs:check` (`metadata`, `links`, `asbuilt-paths`, `index-freshness`), `docs/governance/docs-check.baseline.json` baseline shrink, `scripts/docs/docs-check.mjs --update-baseline`.

---

## File Map

- `docs/archive/**` — new archive destinations (one subdirectory per topical group: `archive/runbooks/`, `archive/specs/`, etc., reusing existing buckets where they apply).
- `docs/archive/README.md` — extend the existing archive landing index if present, or create one, listing newly archived documents with a one-line reason and replacement pointer.
- `docs/runbooks/release-runbook.md` *or* `docs/guides/release-runbook.md` — final location after migration-exception resolution; the legacy path either disappears or becomes a one-paragraph redirect stub.
- `README.md`, `AGENTS.md`, `CLAUDE.md` — contributor entrypoints relinked to `docs/indexes/README.md` and topical READMEs.
- `docs/README.md` — top-level docs landing rewritten to point through generated indexes; public-vs-internal platform claims reconciled to current reality.
- `docs/guides/documentation-governance.md` — release-runbook migration exception line removed once closed; public-vs-internal claim aligned with `docs/README.md`.
- `packages/*/README.md`, `crates/*/README.md` — cross-references to archived docs replaced with current pointers; broken intra-repo links fail `pnpm docs:check`.
- `docs/governance/docs-check.baseline.json` — shrinks as archived docs leave the active set and as entrypoint relinking removes stale references.
- `plans/archive/**` — destination for any APS-adjacent docs whose authority has been replaced by a module/index.
- `plans/modules/documentation-governance.aps.md` — DOCGOV-008 status and closeout.
- `plans/index.aps.md` — DOCGOV progress/status reconciliation when the item closes.

## Tasks

### Task 1: Dead-Doc Audit

**Files:**

- Create: `plans/execution/DOCGOV-008.audit.md` (working artefact, archived or deleted at task close)
- Read: every file under `docs/**` not already in `docs/archive/**`, `docs/indexes/**`, or matching `*.template.md`

- [x] Produce a categorised list of archive candidates: dead spec, superseded runbook, stale guide, unused review, orphaned status note. Include path, last meaningful commit date, why it's dead (superseded by / no inbound links / refers to retired surface), and proposed destination.
- [x] For each candidate, run `rg -l '<basename>' --type md --type rs --type ts` to find inbound links; classify as: (a) no inbound links → simple archive, (b) inbound links inside repo → relink callers in same change, (c) inbound links outside repo (release notes, external docs, blog posts) → archive with redirect stub.
- [x] Hand the categorised list to the operator for sign-off before any moves. Do not `git mv` anything in this task.
- [x] Stop the task on operator approval; deletions and moves happen in Task 3.

### Task 2: Resolve Release-Runbook Migration Exception

**Operator decision (locked):** **Option A** — relocate `docs/guides/release-runbook.md` to `docs/runbooks/release-runbook.md`. The Option B alternative (archive-with-stub) is **not** in scope for this task.

**Files:**

- Move: `docs/guides/release-runbook.md` → `docs/runbooks/release-runbook.md`
- Modify: `docs/guides/documentation-governance.md` (drop the migration-exception note)
- Modify: any inbound link that references `docs/guides/release-runbook.md` (`README.md`, `AGENTS.md`, `docs/README.md`, package READMEs, runbook cross-references)

- [x] `git mv docs/guides/release-runbook.md docs/runbooks/release-runbook.md`; preserve history.
- [x] Update every inbound link discovered by `rg -l 'docs/guides/release-runbook'` so `pnpm docs:check` link surface stays green.
- [x] Update the relocated runbook's `Upstream`/`Downstream` table only if any of its referenced paths drift as a result of the move; the Freshness anchor itself stays unchanged.
- [x] Remove the "Current migration exception" paragraph in `docs/guides/documentation-governance.md:52-54`.
- [x] Run `pnpm docs:check` and verify `links` surface reports zero new errors.

### Task 3: Archive Dead Docs

**Files:**

- Move: each approved candidate from Task 1 via `git mv <src> docs/archive/<bucket>/<basename>` (or `plans/archive/**` where the file is APS-adjacent).
- Modify: `docs/archive/README.md` — index the new archived entries with one-line reasons.
- Create: redirect stubs at the original path *only* for documents Task 1 flagged as having external inbound links. Stub content: H1 + one-paragraph pointer + `Status: Archived` metadata table.
- Modify: any in-repo callers whose links Task 1 flagged for relinking.

- [x] For each candidate, `git mv` to the destination chosen in Task 1.
- [x] Relink in-repo callers in the same commit so `pnpm docs:check` links surface stays green.
- [x] Add redirect stubs at the original paths only where Task 1 flagged external inbound links; stub uses the DOCGOV-002 governance table with `Status: Archived`.
- [x] Run `pnpm docs:check`; baseline shrinkage is expected — record the delta in the task notes.
- [x] Do not run `--update-baseline` yet; that happens at closeout to capture the combined Task 3 + Task 4 + Task 5 delta in one place.

### Task 4: Relink Contributor Entrypoints

**Files:**

- Modify: `README.md`, `AGENTS.md`, `CLAUDE.md`
- Modify: `docs/README.md`
- Modify: `packages/*/README.md`, `crates/*/README.md` where they list docs surfaces inline

- [x] Rewrite the "docs" section of each entrypoint to route through `docs/indexes/README.md` + topical READMEs rather than enumerating surfaces inline. The goal is one link to discovery, not a flat list that drifts.
- [x] Remove links to any document moved in Task 3 (relink to its replacement or to the index that supersedes it).
- [x] Keep package and crate READMEs scoped to their own surface; cross-cutting discovery belongs in `docs/indexes/`.
- [x] Run `pnpm docs:check` and verify `links` surface remains green.

### Task 5: Reconcile Public-vs-Internal Platform Claims

**Files:**

- Modify: `docs/README.md`
- Modify: `docs/guides/documentation-governance.md`

- [x] Audit the platform claims in `docs/README.md` and `docs/guides/documentation-governance.md` against current reality (where public docs actually publish, who owns them, what the contract is). Capture findings inline as bullets in the file's "what's true today" section; do not introduce new prose elsewhere.
- [x] Where a claim is wrong, fix it. Where a claim is aspirational, mark it as such with an inline `TODO: APS link` and create or link an APS work item.
- [x] Run `pnpm format:check` and `pnpm docs:check`.

### Task 6: Governance Closeout

**Files:**

- Modify: `docs/governance/docs-check.baseline.json`
- Modify: `docs/indexes/**` (regenerated)
- Modify: `plans/modules/documentation-governance.aps.md`
- Modify: `plans/index.aps.md`

- [x] Run `pnpm docs:index` to regenerate generated indexes against the post-archive corpus.
- [x] Run `pnpm docs:check:update-baseline` to capture the combined baseline shrink from Tasks 3–5.
- [x] Confirm `pnpm docs:check` reports 0 new errors and the baseline has shrunk; record the delta (per-surface counts before/after) in the DOCGOV-008 task body.
- [x] Mark DOCGOV-008 complete only after `pnpm docs:check && pnpm docs:index:check && pnpm format:check && pnpm lint:check && pnpm test:docs-check` pass.
- [x] Update `plans/index.aps.md` DOCGOV progress counter (8/10) and module narrative.

**Closeout note:** This branch completed Tasks 1–6 and regenerated the baseline / indexes for Task 6. Fresh closeout validation passed on 2026-05-24 with `pnpm docs:check && pnpm docs:index:check && pnpm format:check && pnpm lint:check && pnpm test:docs-check`. DOCGOV-008 is now closed in APS; DOCGOV-009 is unblocked.
