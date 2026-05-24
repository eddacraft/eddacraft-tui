# DOCGOV-009 Implementation Plan

**Goal:** Backfill the DOCGOV-002 governance metadata block onto every live document under `docs/**` so the entire active corpus declares Type / Authority / Owner / Status / Freshness on the same contract, the `docs-check.baseline.json` baseline shrinks toward zero, generated indexes cover the full active corpus, and tags resolve through the approved catalogue.
**Architecture:** Sweep by authority and risk, highest-impact first. High-authority surfaces (architecture, as-built, runbooks, guides, policies) get human-curated freshness anchors against a specific tag/SHA/source-path. Public docs anchor against the release/version they describe. Marketing/vision/strategy/spec surfaces anchor against their owning APS module or RELEASE-PLAN entry. The generator and validators already exist (DOCGOV-005/006/007); this work item is metadata authorship, not new tooling. DOCGOV-008 must complete first so dead docs do not get governed in place.
**Tech Stack:** Markdown metadata tables, `@eddacraft/anvil-docs-meta` parser, `pnpm docs:check` (metadata + tags + index-freshness + asbuilt-paths surfaces), `docs/governance/docs-check.baseline.json` baseline shrink, `docs/governance/tags-catalogue.md` tag gate, generated indexes under `docs/indexes/`.

---

## File Map

- `docs/architecture/**/*.md` — as-built and architecture references; freshness anchors tag/SHA + source paths.
- `docs/runbooks/**/*.md` — operational procedures; freshness anchors last successful dry-run/release/incident + executable script paths.
- `docs/guides/**/*.md` — developer-practice guides; freshness anchors upstream rule, APS item, ADR, or source path.
- `docs/policies/**/*.md` — operational policy; freshness anchors the policy review or APS authority.
- `docs/public/**/*.md` — user-facing behaviour; freshness anchors release/product version.
- `docs/marketing/**/*.md`, `docs/strategy/**/*.md`, `docs/vision/**/*.md` — high-judgement freshness anchors against owning module / RELEASE-PLAN entry / explicitly-noted aspirational status.
- `docs/specs/**/*.md` — design specs; freshness anchors APS module they feed or supersede date.
- `docs/observability/**/*.md`, `docs/testing/**/*.md`, `docs/internal/**/*.md`, `docs/reviews/**/*.md`, `docs/plans/**/*.md` — ancillary surfaces; freshness anchors most-recent canonical source.
- `docs/governance/tags-catalogue.md` — extended only when backfill surfaces a tag that should join the catalogue.
- `docs/governance/docs-check.baseline.json` — shrinks per task as backfilled files leave the baseline.
- `docs/indexes/**` — regenerated at end of each task.
- `plans/modules/documentation-governance.aps.md` — DOCGOV-009 status and closeout.
- `plans/index.aps.md` — DOCGOV progress/status reconciliation when the item closes.

## Tasks

### Task 1: Owner & Freshness Rubric

**Files:**

- Create: `plans/execution/DOCGOV-009.rubric.md` (working artefact; archived or deleted at task close)

- [x] Define owner mapping: how to choose `Owner` when a doc has no obvious APS module. Default rubric: prefer APS module ID for active work; fall back to team handle (`@aneki`, `RELORCH`, `DOCGOV`, etc.); use `Docs governance` only for derived/generated surfaces.
- [x] Define freshness mapping per `Type`, restating the rules in `docs/guides/documentation-governance.md:115-126` with concrete example anchors per directory (e.g. `docs/runbooks/release-signing.md` → "Last reviewed YYYY-MM-DD against `scripts/release/sign-artefacts.sh` and `v0.7.1-beta`").
- [x] Define authority mapping per surface (`Authoritative` for primary sources, `Derived` for indexes / generated, `Advisory` for guides without enforcement, `Historical` for never-archive content kept in active paths).
- [x] Resolve ambiguous cases by listing them; ambiguous docs go into Task 4 (judgement bucket) rather than the high-authority sweep.
- [x] Hand the rubric to the operator for sign-off before any backfill writes start.

### Task 2: Backfill High-Authority Surfaces

**Files:**

- Modify: every `docs/architecture/**/*.md` not already governed (architecture references + as-built docs)
- Modify: every `docs/runbooks/**/*.md` not already governed
- Modify: every `docs/guides/**/*.md` not already governed
- Modify: every `docs/policies/**/*.md` not already governed

- [ ] Walk the list of files under these directories that still appear in `docs/governance/docs-check.baseline.json` (`metadata` surface). For each, add the DOCGOV-002 metadata table immediately after the H1, plus the Upstream/Downstream relationships table.
- [ ] For as-built and runbook docs, ensure every backtick-wrapped path in the freshness/upstream/downstream/body resolves on disk — the `asbuilt-paths` surface will fail otherwise. Use `git ls-files` to verify.
- [ ] Add or move any `**Tags:**` line so it sits below the relationships table and uses only catalogue values from `docs/governance/tags-catalogue.md`. Unknown tags either become catalogue additions (in the same change, with rationale) or get dropped.
- [ ] After each batch of ~10 files, run `pnpm docs:check` and confirm the baseline has shrunk without introducing new findings. Do not run `--update-baseline` per-batch; that happens once per task.
- [ ] At task end: `pnpm docs:check:update-baseline`, `pnpm docs:index`, `pnpm docs:check`, `pnpm format:check`. Record per-surface baseline delta in DOCGOV-009 task notes.

**Task 2 batch notes:**

- 2026-05-25 batch 1 backfilled five high-authority guides:
  `docs/guides/adr-process.md`, `docs/guides/branching-strategy.md`,
  `docs/guides/worktree-policy.md`,
  `docs/guides/feature-flag-governance.md`, and `docs/guides/testing.md`.
  `pnpm docs:check` initially failed only because generated indexes were stale;
  `pnpm docs:index && pnpm docs:check && pnpm docs:index:check && pnpm format:check`
  then passed. Baseline update intentionally deferred until Task 2 closeout.
- 2026-05-25 batch 2 backfilled five more high-authority guides:
  `docs/guides/agent-surface-inventory.md`,
  `docs/guides/cli-output-streams.md`, `docs/guides/command-safety.md`,
  `docs/guides/command-safety-configuration.md`, and
  `docs/guides/custom-architecture-policies.md`. Generated indexes were
  refreshed, and
  `pnpm docs:index && pnpm docs:check && pnpm docs:index:check && pnpm format:check`
  passed after formatting. Baseline update intentionally deferred until Task 2
  closeout.
- 2026-05-25 batch 3 backfilled five high-authority guides:
  `docs/guides/edda-memory.md`,
  `docs/guides/feature-flag-inventory.md`,
  `docs/guides/feature-flag-reference.md`,
  `docs/guides/git-hook-compatibility.md`, and
  `docs/guides/opa-policy-testing.md`. Generated indexes were refreshed.
  `pnpm docs:check && pnpm docs:index:check && pnpm format:check` passed after
  applying `pnpm format`. `docs/guides/eddacraft-autonomy-constitution.md` was
  left for Task 4 judgement because its draft-operational status needs owner and
  authority confirmation rather than a high-authority sweep guess. Baseline
  update intentionally deferred until Task 2 closeout.
- 2026-05-25 batch 4 backfilled three remaining clear guide surfaces:
  `docs/guides/adapters/workflow-guide.md`,
  `docs/guides/ember-candidates.md`, and `docs/guides/stack-migration.md`.
  Generated indexes were refreshed, and
  `pnpm docs:check && pnpm docs:index:check && pnpm format:check` passed.
  `docs/guides/anchor-rescoring-process.md` was left for Task 4 judgement
  because the guide itself states that it has no permanent named owner. Baseline
  update intentionally deferred until Task 2 closeout.
- 2026-05-25 batch 5 backfilled two clear non-as-built architecture references:
  `docs/architecture/quality-model.md` and
  `docs/architecture/references/entire-branch-sidecar.md`. Generated indexes
  were refreshed, and
  `pnpm format && pnpm docs:index && pnpm docs:check && pnpm docs:index:check && pnpm format:check`
  passed. Stale or superseded-looking architecture specs and PocketFlow
  references were left for Task 4 judgement rather than assigned authority during
  the high-authority sweep. Baseline update intentionally deferred until Task 2
  closeout.
- 2026-05-25 batch 6 backfilled one low-risk runbook:
  `docs/runbooks/observability-triage.md`. The inline severity labels were
  changed from backtick-wrapped code spans to plain text so the governed runbook
  does not introduce source-path validation noise. Generated indexes were
  refreshed, and
  `pnpm format && pnpm docs:index && pnpm docs:check && pnpm docs:index:check && pnpm format:check`
  passed. Baseline update intentionally deferred until Task 2 closeout.

### Task 3: Backfill Public Documentation

**Files:**

- Modify: every `docs/public/**/*.md` not already governed (≈35 files per baseline)

- [ ] Apply the rubric: `Type: Public docs`, `Authority: Authoritative` (for product-behaviour docs) or `Authority: Derived` (for index/overview pages that summarise other public surfaces).
- [ ] Freshness anchor: the release/product version the doc describes (e.g. `v0.7.1-beta`). Where a public doc covers multiple versions, anchor against the most recent one and list the others in `Upstream`.
- [ ] `Owner` defaults to the team handle that owns public-docs publishing; confirm with operator if no clear owner exists.
- [ ] Tags must come from the catalogue; public-docs sweeps frequently surface tag-catalogue additions — surface them in PR description, not via inline edits without rationale.
- [ ] After every ~10 files: `pnpm docs:check` for delta. End of task: `pnpm docs:check:update-baseline && pnpm docs:index && pnpm docs:check && pnpm format:check`.

### Task 4: Backfill Ancillary & Judgement Surfaces

**Files:**

- Modify: every `docs/marketing/**/*.md`, `docs/vision/**/*.md`, `docs/strategy/**/*.md`, `docs/specs/**/*.md` not already governed
- Modify: every `docs/observability/**/*.md`, `docs/testing/**/*.md`, `docs/internal/**/*.md`, `docs/reviews/**/*.md`, `docs/plans/**/*.md` not already governed
- Modify: any remaining baselined doc not covered by Tasks 2–3

- [ ] Apply the rubric, using `Advisory` or `Historical` where `Authoritative` does not fit (e.g. brainstorm output, retired status notes still in active paths).
- [ ] Any document where ownership or freshness is genuinely unclear gets flagged back to the operator rather than guessed. Two paths in those cases: (a) operator supplies an anchor → backfill in this task, (b) operator says "this is dead" → route back to DOCGOV-008 archival.
- [ ] Tags from this sweep are likely to push the catalogue; same rule applies — catalogue updates in same change, with rationale.
- [ ] End of task: `pnpm docs:check:update-baseline && pnpm docs:index && pnpm docs:check && pnpm format:check`.

### Task 5: Tag Catalogue Reconciliation

**Files:**

- Modify: `docs/governance/tags-catalogue.md`
- Modify: any backfilled file whose tag list contains an unapproved tag at the start of this task

- [ ] Aggregate the catalogue additions and tag drops accumulated across Tasks 2–4. Group additions by Domain / Activity / Lifecycle per the existing catalogue structure.
- [ ] For each new tag, add a one-line `Intent` row and link the work item where the tag was first used. Reject tags that exist only in dead docs (those should already be archived).
- [ ] Verify with `pnpm docs:check` `tags` surface — only the baselined warning entries (≤1 today) should remain or shrink.
- [ ] If any backfilled file still uses a tag the catalogue rejects, fix the file (not the catalogue).

### Task 6: Governance Closeout

**Files:**

- Modify: `docs/governance/docs-check.baseline.json`
- Modify: `docs/indexes/**` (regenerated)
- Modify: `plans/modules/documentation-governance.aps.md`
- Modify: `plans/index.aps.md`

- [ ] Run `pnpm docs:check:update-baseline` to capture the full-corpus baseline state after Tasks 2–5.
- [ ] Expected state: `metadata` baseline = 0 (or limited to documented exceptions, with the exception list in DOCGOV-009 task notes); `tags` baseline ≤ pre-task value; `links` and `asbuilt-paths` unchanged or shrunk.
- [ ] Run `pnpm docs:index` and confirm generated indexes now cover the full active corpus (every Task 2–4 backfill is visible).
- [ ] Record per-surface before/after baseline counts in DOCGOV-009 task body.
- [ ] Mark DOCGOV-009 complete only after `pnpm docs:check && pnpm docs:index:check && pnpm format:check && pnpm lint:check && pnpm test:docs-check` pass.
- [ ] Update `plans/index.aps.md` DOCGOV progress counter (9/10) and module narrative.
- [ ] Confirm DOCGOV-010 dependencies are now satisfied (DOCGOV-005, -008, -009 all Done).
