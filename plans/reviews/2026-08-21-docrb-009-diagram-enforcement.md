# DOCRB-009 diagram enforcement evidence

**Date:** 2026-08-23
**Item:** DOCRB-009
**Pinned starting receipt:** `89a6d2050b7a93e69ac6ea99848d9dfa7c63be3c`
**Rebased base:** `cec7b0d3157b52920474a5a6350ec480a9b86fc7`
**Result:** Executor repair GREEN after rebase; fresh exact-head independent re-verification and targeted Council closure remain lifecycle gates.

## Scope and authority

The operator approved a final 35-path ceiling: four policy and guide paths,
three pinned-toolchain paths, four docs-check implementation/test paths, seven
existing CI/local routing and contract-test paths, two APS lifecycle paths,
the action plan, thirteen directly triggered freshness/review metadata
closeouts, and this evidence report.

The approved ceiling remains 35. After the initial rebase, one closeout had
already been independently satisfied upstream and the range contained 34
paths. The 2026-08-23 review-date rollover made that closeout branch-relevant
again, so the final range uses the exact approved 35 paths without adding a new
scope path.

The thirteen closeouts are `apps/docs-shell/ARCHITECTURE.md`,
`docs/README.md`, `docs/architecture/README.md`,
`docs/architecture/docs-delivery.md`, `docs/architecture/overview.md`,
`docs/architecture/trust-and-deployment-boundaries.md`,
`docs/governance/tags-catalogue.md`, `docs/guides/README.md`,
`docs/guides/adapters/README.md`,
`docs/reviews/shipped-codebase-review-checklist.md`,
`docs/guides/testing.md`, `docs/reviews/README.md`, and
`plans/specs/2026-08-19-anvil-docs-definition-layer.md`. Each closeout changes
Freshness metadata only. Its owning prose, diagrams, authority, links,
navigation, and product claims remain unchanged. Current `main` had already
refreshed `docs/reviews/shipped-codebase-review-checklist.md` in the initial
rebase range. The 2026-08-23 date rollover made that closeout branch-relevant
again, so it is present as the final feature range's 35th path.

`apps/docs-shell/README.md` remains unchanged: it is outside the current
docs-owed and Mermaid gates, and manual review found its production-topology
and ownership prose compatible. Four inherited baselined freshness findings
also remain unchanged. DOCRB-010, public information architecture/content,
diagram content, Draw.io tooling, DOCFRESH mechanics/baselines, release files,
generated indexes, product runtime code, and new CI jobs/statuses are outside
scope.

## Contract and TDD evidence

- Initial RED: `node --test scripts/docs/check-diagram-impact.test.mjs` exited
  1 with `ERR_MODULE_NOT_FOUND` for the new checker. The first GREEN passed
  5/5 extraction, upstream matching, relevant-owner failure, updated-owner
  success, and irrelevant-change cases.
- Renderer slices progressed through missing-export REDs to injected-renderer,
  orchestration, repository-local CLI, and exact-error fallback GREEN.
- Discovery RED proved component `Type: Architecture` metadata was skipped;
  the narrow five-column governance-table fallback made the live component
  corpus visible without changing component documents.
- Plan discovery RED proved governed `plans/specs/**` Mermaid was omitted.
  GREEN includes the retained definition-layer spec and its seven fences.
- Exact-version REDs covered mismatched, ambiguous, stderr-bearing, and
  non-zero version results. GREEN requires stdout exactly `11.16.0` and
  fails closed before publishing the CLI summary.
- Final focused checker suite: 16/16 passed after the binding Council repair.
- Classifier RED lacked `diagram-impact`; the final GREEN selects the existing
  signal for every non-empty trusted change set. The semantic checker remains
  the relevance filter, and an unrelated change passes it without a waiver.
- Local-validator RED rejected the unknown signal; GREEN invokes the checker
  with the existing trusted paths file.
- CI-integration RED lacked the Docs corpus invocation; GREEN carries one
  composite-action output into the existing Docs corpus worker and existing
  Docs Lint status. An unusable diff base now emits
  `Cannot determine diagram-impact diff base` and exits 2 rather than silently
  narrowing to `HEAD~1`.

### Independent-verifier deletion repair

The prior completion evidence did not cover removal of a declared upstream.
Both the checker's real `--since` collector and the automatic local changed and
staged collectors used `--diff-filter=ACMR`, so a deleted exact upstream could
disappear before classification. The earlier completion claim is superseded
for deletion coverage by this repair evidence.

- Checker RED: a real temporary Git repository committed an exact declared
  upstream and then its deletion; the real `--since` CLI returned 0 instead of
  the required `diagram-review-owed` exit 1.
- Local-routing RED: both automatic `--changed` and `--staged` fixtures omitted
  the deleted path and completed successfully instead of invoking the selected
  diagram-impact failure.
- Minimal GREEN: all three collectors now use `--diff-filter=ACDMR`, retaining
  deletions while preserving additions, copies, modifications, and renames.
- Checker GREEN: 14/14 focused tests pass. Local validation fixtures pass for
  both automatic deletion routes.
- Real reproduction: deleting `crates/deletion-repro/src/lib.rs` in a temporary
  repository makes the real CLI exit 1 with `diagram-review-owed` for
  `docs/architecture/deletion-repro.md`; one affected fence renders through
  exact Mermaid 11.16.0 with the conditional sandbox fallback.

### Council db835ed9 binding repairs

Council observations C-001 through C-005 are deduplicated here into two binding
implementation repairs. Session `council-db835ed9` converged on the pre-rebase
content after both repairs; that verdict does not substitute for fresh
exact-head closure after `origin/main` moved.

- Routing RED: `bash scripts/ci/classify-changes.test.sh` exited 1 because the
  governed plan-spec owner selected only `markdownlint`. The same missing
  signal affected the sampled owner and upstream classes, including a deleted
  upstream.
- Routing GREEN: every non-empty trusted changed-path set selects the existing
  cheap `diagram-impact` signal. Empty input remains unselected. The checker,
  rather than a second shell metadata model, decides whether a path affects a
  governed owner; irrelevant changes still return zero findings without a
  waiver. Classifier, automatic local planning, and CI integration fixtures all
  pass.
- Sandbox RED: `node --test scripts/docs/check-diagram-impact.test.mjs` passed
  14/15 tests; attacker-controlled candidate parse text containing
  `No usable sandbox!` incorrectly authorised a second unsandboxed attempt.
- Sandbox GREEN: one fixed trusted Mermaid probe runs normally before candidate
  rendering. Only the anchored full Chromium FATAL line captured on this host,
  with dynamic timestamp and source-line numerics, selects the existing
  temporary two-flag fallback. Candidate prefix, suffix, and mixed parse
  errors never authorise fallback; trusted-probe near-matches fail closed.
  Probe and candidate temporary directories are removed in `finally`.
- Final focused checker result: 16/16 tests pass. Real corpus mode renders 17
  governed documents and 24 Mermaid fences with exact Mermaid CLI 11.16.0,
  conditional fallback mode, and zero findings.

### Hosted review repairs

Two late PR review findings were reproduced and repaired without expanding the
approved path set.

- Local validation RED: a fresh checkout could plan the diagram-impact command
  without first building `@eddacraft/anvil-docs-meta`, whose package entrypoint
  is generated under `dist/`. GREEN adds the existing filtered docs-meta build
  immediately before the checker command; the local-plan fixture proves both
  presence and order. The automatic changed and staged deletion fixtures each
  provide an exact hermetic stub for that build command and require distinct
  build and checker markers, proving their expected non-zero result comes from
  the diagram check rather than its prerequisite.
- Renderer summary RED: a diff with no affected Mermaid fences reported
  `sandboxed` even though the trusted probe had not run. GREEN reports
  `not-probed` until the fixed probe detects a mode, then reports that detected
  `sandboxed` or `conditional-no-sandbox-fallback` value. Focused checker and
  local-validator fixtures pass.

## Mermaid runtime and corpus

- Root development dependency: exact `@mermaid-js/mermaid-cli@11.16.0`.
- The only new build allowance is `puppeteer: true`.
- Installed CLI readback: stdout exactly `11.16.0`; stderr empty.
- Puppeteer installed Chrome and chrome-headless-shell `152.0.7977.42`; Firefox
  was skipped by its upstream installer.
- The fixed trusted probe's normal browser launch runs first. On this host it
  fails at the nested AppArmor boundary with the anchored canonical Chromium
  `No usable sandbox!` FATAL line.
- Only that trusted-probe signature triggers the repository-established
  temporary Puppeteer configuration
  `{args:["--no-sandbox","--disable-setuid-sandbox"]}`. The config, Mermaid
  input, and SVG output live in temporary directories removed in `finally`.
  Candidate content and errors never choose the mode; other launch and render
  failures do not retry and remain fail-closed.
- Corpus mode: 17 governed documents, 24 Mermaid fences, 0 findings.
- Final diff mode: 17 governed documents discovered, 11 affected fences
  rendered, 0 findings.
- Existing public parity remains authoritative:
  `pnpm docs:public:diagrams` reports 4 governed files, 0 errors, 0 warnings.

## Freshness and generated-output closeout

The pre-rebase committed-head simulation of the complete 34-path
pre-evidence patch reported 0 errors, 0 unbaselined findings, and 0 advisory
findings. After rebasing, the exact range against current `origin/main`
reports the same four inherited baselined gating warnings and no
diff-attributable freshness error:

- `docs/runbooks/main-first-cutover.md` from `.github/workflows/ci.yml`;
- `docs/runbooks/branch-reconciliation.md` from `.github/workflows/ci.yml`;
- `docs/guides/repository-operations.md` from `AGENTS.md`;
- `docs/guides/agent-surface-inventory.md` from `AGENTS.md`.

The reverse-edge audit found no further exact-file consumer. The tags catalogue
feeds generated `docs/indexes/by-tag.md`, but Freshness is not rendered into
that index and generated indexes are outside docs-owed. `pnpm docs:index:check`
passes with 6 generated files, 0 errors, and 0 warnings; no generated file
changed.

## Validation evidence

Fresh pre-evidence results:

- `node --test scripts/docs/check-diagram-impact.test.mjs` — 16/16 passed.
- Diagram-impact diff and corpus modes — GREEN with exact Mermaid 11.16.0.
- `pnpm test:ci-classify` — GREEN.
- `pnpm test:validate-local` — GREEN, including automatic changed and staged
  deletion retention; the fixture's invalid-shell line and inherited
  `/tmp/anvil-test/env` profile warnings are expected.
- `pnpm test:ci-integration` — GREEN.
- `pnpm docs:public:diagrams` — 4 files, 0 errors, 0 warnings.
- `pnpm docs:index:check` — 6 files, 0 errors, 0 warnings.
- `pnpm format:check` — 1,686 files clean.
- Markdown lint and changed shell syntax — GREEN.
- `pnpm lint:check` — GREEN after an unsandboxed rerun allowed Nx to write its
  cache; the initial linked-Worktrunk run failed only with `EROFS`.
- `pnpm aps:active-lint` — 147 active files clean.
- `pnpm aps:index:check` — exit 0; DOCRB remains consistent at stored 9/11.
  Current `main` contributes an advisory GTAO header/index 0/10 versus task
  count 3/10 mismatch.
- `pnpm aps:drift --json` — exit 0; inherited current-main warnings report the
  same GTAO progress mismatch and `IMPV-001` Complete without explicit
  validation evidence, with no DOCRB finding.
- `git diff --check` — GREEN.

Before rebase, `pnpm docs:check` exposed the new non-baselineable
`diagram-impact` surface as passing while the inherited release-plan surface
failed. Current `main` advanced its active window to untagged
`v0.9.8-beta`; after rebase, all 13/13 documentation surfaces pass. The
fresh unsandboxed `pnpm test:docs-check` run also passes all cases, including
95 Node tests and the deliberate unreadable-corpus fixture. The earlier
linked-Worktrunk sandbox run that reached that fixture's `chmod` and reported
`EROFS` remains environment-only. The fresh unsandboxed `pnpm lint:check`
run passes across the TypeScript/JavaScript, Markdown, and 37-project Rust
lint/format surfaces with inherited warnings only.

## Boundary decision

The repaired implementation satisfies the mandatory remove/update-or-unaffected contract,
exact Mermaid rendering, declared-upstream review, public parity composition,
and one fail-closed routing signal without adding a waiver marker, parser for
PR prose, CI job, or required status. Publication remains blocked on fresh
exact-head verification and Council review.
