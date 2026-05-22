# DOCGOV-007 Implementation Plan

**Goal:** Generate deterministic documentation indexes from governed document metadata and fail `pnpm docs:check` when generated indexes are stale.
**Architecture:** Keep metadata parsing in `@eddacraft/anvil-docs-meta` and implement index generation in a repository-local Node ESM script. The generator scans governed Markdown documents, emits generated Markdown under `docs/indexes/`, and `check-index-freshness.mjs` delegates to the same generator in check mode so `pnpm docs:check` remains the single closeout command.
**Tech Stack:** Node ESM scripts, `@eddacraft/anvil-docs-meta`, `globby`, Markdown generated artefacts, package scripts.

---

## File Map

- `scripts/docs/docs-index.mjs` — new generator/check command for deterministic documentation indexes.
- `scripts/docs/check-index-freshness.mjs` — replace the DOCGOV-007 stub with a wrapper around `docs-index.mjs --check`.
- `scripts/docs/docs-check.test.sh` — update contract tests for the real index-freshness surface.
- `package.json` — add `docs:index` and `docs:index:check` scripts.
- `docs/indexes/README.md` — generated index landing page with marker header.
- `docs/indexes/by-type.md` — generated type index.
- `docs/indexes/by-authority.md` — generated authority index.
- `docs/indexes/by-owner.md` — generated owner index.
- `docs/indexes/by-status.md` — generated status index.
- `docs/indexes/by-tag.md` — generated tag index, initially populated from approved tag usage where available.
- `docs/guides/documentation-governance.md` — document the generated-index location and manual-edit rule.
- `docs/governance/tags-catalogue.md` — update only if generator needs a DOCGOV-owned index tag.
- `plans/modules/documentation-governance.aps.md` — DOCGOV-007 state and closeout.
- `plans/index.aps.md` — DOCGOV progress/status reconciliation when the item closes.

## Tasks

### Task 1: Generator Contract

**Files:**
- Create: `scripts/docs/docs-index.mjs`
- Modify: `package.json`
- Test: `scripts/docs/docs-check.test.sh`

- [x] Add failing contract coverage for `pnpm docs:index:check` missing/stale generated files.
- [x] Run `pnpm test:docs-check` and verify the expected failure.
- [x] Implement metadata scanning, deterministic sort, generated marker, and `--check` diff detection.
- [x] Run `pnpm docs:index:check` and verify it reports missing generated indexes before generation.

### Task 2: Generated Index Artefacts

**Files:**
- Create: `docs/indexes/README.md`
- Create: `docs/indexes/by-type.md`
- Create: `docs/indexes/by-authority.md`
- Create: `docs/indexes/by-owner.md`
- Create: `docs/indexes/by-status.md`
- Create: `docs/indexes/by-tag.md`

- [x] Run `pnpm docs:index` to write generated files.
- [x] Run `pnpm docs:index:check` and verify no stale generated files.
- [x] Confirm generated files contain no hand-authored authority prose beyond the generated marker.

### Task 3: Docs-Check Integration

**Files:**
- Modify: `scripts/docs/check-index-freshness.mjs`
- Modify: `scripts/docs/docs-check.mjs`
- Modify: `scripts/docs/docs-check.test.sh`

- [x] Replace the pending DOCGOV-007 stub with `docs-index.mjs --check` delegation.
- [x] Add JSON/summary contract coverage for the real index-freshness surface.
- [x] Run `pnpm docs:check && pnpm test:docs-check`.

### Task 4: Governance Closeout

**Files:**
- Modify: `docs/guides/documentation-governance.md`
- Modify: `plans/modules/documentation-governance.aps.md`
- Modify: `plans/index.aps.md`

- [x] Document `docs/indexes/` as generated discovery surfaces.
- [x] Mark DOCGOV-007 complete only after generated-index, docs-check, format, lint, and package tests pass.
- [x] Run `pnpm docs:index:check && pnpm docs:check && pnpm test:docs-check && pnpm format:check && pnpm lint:check`.
