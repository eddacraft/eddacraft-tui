# Steps: DOCGOV-005

| Field  | Value                                                                                          |
| ------ | ---------------------------------------------------------------------------------------------- |
| Source | [../modules/documentation-governance.aps.md](../archive/modules/documentation-governance.aps.md)       |
| Task   | DOCGOV-005 — Add documentation validation baseline                                             |
| Status | Draft                                                                                          |
| ADR    | [ADR-042](../decisions/042-closeout-enforcement-exit-codes.md) — closeout-enforcement carve-out |

## Goal

Replace memory-based closeout hygiene with `pnpm docs:check`: a labelled,
baseline-aware orchestrator over seven validation surfaces (five real, two
stub) that fails CI only on net-new violations.

## Prerequisites

- [ ] DOCGOV-002 metadata convention shipped (five-column metadata + Upstream/Downstream table)
- [ ] DOCGOV-004 ADR integrity script + `pnpm adr:check` available
- [ ] `pnpm aps:drift` available
- [ ] Planning Council outcomes recorded (this file + ADR-042)

## Phase 1 — Foundations

### 1. Write ADR-042 carve-out

- **Purpose:** Resolve the ADR-002 vs hard-fail contradiction in writing.
- **Produces:** `plans/decisions/042-closeout-enforcement-exit-codes.md`; DECISION-LOG row.
- **Checkpoint:** ADR-042 file exists; DECISION-LOG indexes it.
- **Validate:** `pnpm adr:check`

### 2. Scaffold `@eddacraft/anvil-docs-meta` package

- **Purpose:** House the metadata parser separately so DOCGOV-006/-007 reuse it.
- **Produces:** `packages/docs-meta/{package.json,project.json,tsconfig.json,vitest.config.ts}`; empty `src/index.ts`.
- **Checkpoint:** `pnpm -F @eddacraft/anvil-docs-meta build` succeeds on empty source.
- **Validate:** `pnpm -F @eddacraft/anvil-docs-meta build`

### 3. Define Zod schema for documentation metadata

- **Purpose:** Capture the five-column metadata table and Upstream/Downstream table shape.
- **Produces:** Zod schemas exported from `@eddacraft/anvil-docs-meta`.
- **Checkpoint:** Schema accepts valid metadata, rejects malformed input.
- **Validate:** `pnpm -F @eddacraft/anvil-docs-meta test`

### 4. Implement metadata parser

- **Purpose:** Extract metadata + relationships from a Markdown file deterministically.
- **Produces:** Parser function returning typed result or structured errors.
- **Checkpoint:** Parser round-trips fixture docs; surfaces errors with file:line.
- **Validate:** `pnpm -F @eddacraft/anvil-docs-meta test`

### 5. Parser fixture coverage

- **Purpose:** Lock parser behaviour against representative real and malformed docs.
- **Produces:** Fixture set under `packages/docs-meta/test/fixtures/`.
- **Checkpoint:** Fixtures cover happy path, missing rows, unknown columns, malformed tables.
- **Validate:** `pnpm -F @eddacraft/anvil-docs-meta test`

## Phase 2 — Per-surface validators

### 6. Metadata validator surface

- **Purpose:** Verify every governed doc declares required metadata.
- **Produces:** `scripts/docs/check-metadata.mjs` consuming `@eddacraft/anvil-docs-meta`.
- **Checkpoint:** Validator reports missing/malformed metadata per file.
- **Validate:** `node scripts/docs/check-metadata.mjs`

### 7. Tags validator surface

- **Purpose:** Warn on unknown tags, error on malformed tag syntax.
- **Produces:** `scripts/docs/check-tags.mjs` reading `docs/governance/tags-catalogue.md`.
- **Checkpoint:** Validator distinguishes warning from error exit per surface contract.
- **Validate:** `node scripts/docs/check-tags.mjs`

### 8. Internal-link validator surface

- **Purpose:** Resolve every cross-doc link by file path and heading anchor.
- **Produces:** `scripts/docs/check-links.mjs` scoped to `docs/**/*.md` and `plans/**/*.md`.
- **Checkpoint:** Validator flags broken file links and missing heading anchors.
- **Validate:** `node scripts/docs/check-links.mjs`

### 9. APS/index consistency delegate

- **Purpose:** Reuse the existing drift checker as a docs:check sub-surface.
- **Produces:** Thin wrapper invoking `pnpm aps:drift` with labelled output.
- **Checkpoint:** Sub-check forwards `aps:drift` exit code under the `aps` label.
- **Validate:** `node scripts/docs/check-aps.mjs`

### 10. ADR integrity delegate

- **Purpose:** Reuse `pnpm adr:check` as a docs:check sub-surface.
- **Produces:** Thin wrapper invoking `adr-integrity.sh` with labelled output.
- **Checkpoint:** Sub-check forwards `adr:check` exit code under the `adr` label.
- **Validate:** `node scripts/docs/check-adr.mjs`

### 11. Generated-index freshness stub

- **Purpose:** Reserve the surface slot until DOCGOV-007 implements it.
- **Produces:** `scripts/docs/check-index-freshness.mjs` no-op stub.
- **Checkpoint:** Stub logs "pending DOCGOV-007" and exits 0.
- **Validate:** `node scripts/docs/check-index-freshness.mjs`

### 12. As-built source path existence stub

- **Purpose:** Reserve the surface slot until DOCGOV-006 implements it.
- **Produces:** `scripts/docs/check-asbuilt-paths.mjs` no-op stub.
- **Checkpoint:** Stub logs "pending DOCGOV-006" and exits 0.
- **Validate:** `node scripts/docs/check-asbuilt-paths.mjs`

## Phase 3 — Orchestrator, baseline, tag catalogue

### 13. Implement docs-check orchestrator

- **Purpose:** Run every sub-check, label output, summarise, aggregate exit code.
- **Produces:** `scripts/docs/docs-check.mjs` spawning each surface as a subprocess.
- **Checkpoint:** All seven surfaces run regardless of individual failure.
- **Validate:** `node scripts/docs/docs-check.mjs`

### 14. Add labelled-output formatter

- **Purpose:** Make per-surface output greppable and CI-readable.
- **Produces:** Prefix every line with the surface name; unified summary footer.
- **Checkpoint:** Output identifies surface, count, and pass/fail per sub-check.
- **Validate:** `node scripts/docs/docs-check.mjs`

### 15. Implement baseline read/write

- **Purpose:** Apply ADR-003 new-edges-only discipline to docs-check.
- **Produces:** Baseline loader and `--update-baseline` writer in `docs-check.mjs`.
- **Checkpoint:** Violations matching the baseline are skipped; new ones fail.
- **Validate:** `node scripts/docs/docs-check.mjs --update-baseline && node scripts/docs/docs-check.mjs`

### 16. Audit current tag usage

- **Purpose:** Discover the tag corpus before seeding the catalogue.
- **Produces:** Inventory of tags currently used across docs and plans.
- **Checkpoint:** Inventory captures every existing tag with usage count.
- **Validate:** manual review of inventory output

### 17. Seed `tags-catalogue.md`

- **Purpose:** Anchor the tags validator with an authoritative starting set.
- **Produces:** `docs/governance/tags-catalogue.md` listing approved tags + intent.
- **Checkpoint:** Catalogue lists every tag the audit found, none unexplained.
- **Validate:** `node scripts/docs/check-tags.mjs`

### 18. Generate initial docs-check baseline

- **Purpose:** Capture the current violation corpus so CI fails only on regressions.
- **Produces:** `docs/governance/docs-check.baseline.json` covering all surfaces.
- **Checkpoint:** Fresh checkout of HEAD passes `pnpm docs:check` with this baseline.
- **Validate:** `node scripts/docs/docs-check.mjs --update-baseline && node scripts/docs/docs-check.mjs`

## Phase 4 — Wire-up, tests, closeout

### 19. Register `pnpm docs:check` and `pnpm test:docs-check`

- **Purpose:** Make the orchestrator discoverable via the standard interface.
- **Produces:** Script entries in root `package.json`.
- **Checkpoint:** Both scripts resolve from a clean install.
- **Validate:** `pnpm docs:check && pnpm test:docs-check`

### 20. Author docs-check fixture tests

- **Purpose:** Lock behaviour for each sub-check, the baseline, and labelled output.
- **Produces:** `scripts/docs/docs-check.test.sh` plus fixture corpus.
- **Checkpoint:** Fixtures cover every surface, baseline hit/miss, label format.
- **Validate:** `pnpm test:docs-check`

### 21. Wire docs:check into `Docs Lint` CI job

- **Purpose:** Enforce the baseline against incoming PRs.
- **Produces:** Updated GitHub Actions workflow invoking `pnpm docs:check`.
- **Checkpoint:** CI job runs docs:check after format/lint; fails on regression.
- **Validate:** push branch and observe CI run

### 22. Update DOCGOV-005 task body

- **Purpose:** Close out the module-level work item.
- **Produces:** DOCGOV-005 Status → Complete with Closeout note citing ADR-042.
- **Checkpoint:** Module task records completion and ADR-042 reference.
- **Validate:** `pnpm aps:drift`

### 23. Bump DOCGOV count to 5/8

- **Purpose:** Reflect completion in the module header and index.
- **Produces:** Updated counts in `documentation-governance.aps.md` header and `plans/index.aps.md`.
- **Checkpoint:** Both files show DOCGOV at 5/8 with consistent statuses.
- **Validate:** `pnpm aps:drift`

### 24. Final spec validation

- **Purpose:** Confirm formatting, lint, and docs-check itself all pass on HEAD.
- **Produces:** Clean validation run.
- **Checkpoint:** All four commands exit 0.
- **Validate:** `pnpm format:check && pnpm lint:check && pnpm test:docs-check && pnpm docs:check`

## Completion

- [ ] ADR-042 Proposed, indexed in DECISION-LOG, cited from DOCGOV-005
- [ ] `@eddacraft/anvil-docs-meta` package shipped with parser tests
- [ ] Five real surfaces + two stubs runnable independently and via orchestrator
- [ ] `docs/governance/tags-catalogue.md` seeded from the live corpus
- [ ] `docs/governance/docs-check.baseline.json` captures current violations
- [ ] `pnpm docs:check` and `pnpm test:docs-check` registered and green
- [ ] `Docs Lint` CI job invokes `pnpm docs:check`
- [ ] DOCGOV-005 marked Complete with Closeout note; module count 5/8 in module + index
- [ ] `pnpm format:check && pnpm lint:check && pnpm test:docs-check && pnpm docs:check` all pass
