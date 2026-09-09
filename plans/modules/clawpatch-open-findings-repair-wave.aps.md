<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Clawpatch Open-Findings Repair Wave

| ID       | Owner | Priority | Status      | Progress |
| -------- | ----- | -------- | ----------- | -------- |
| CLAWOPEN | —     | P1       | In Progress | 9/12     |

**Last reviewed:** 2026-09-09 — #4361 merged, so CLAWOPEN-007 is Merged;
CLAWOPEN-011 is Complete now its hosted Neon proof is green on `main`
(run 34305126477). Prior:
CLAWOPEN-002 and CLAWOPEN-010 merged via
[#4219](https://github.com/eddacraft/anvil-001/pull/4219) (`6e728c0f7`). The
bounded first delivery of seven clusters covering 14 of the 24 findings that
remain after SETCON-012 merged via
[#4216](https://github.com/eddacraft/anvil-001/pull/4216) (`965a9e7f4`). The
source set remains the
[2026-08-28 complete-store triage](../../docs/reviews/2026-08-28-clawpatch-open-findings.md)
selected for this repair wave. CLAWOPEN-001, -002, -003, -004, -005, -006, -008,
-007, -008, -010, and -012 are Merged. CLAWOPEN-011 is Complete on its hosted
Neon proof; CLAWOPEN-009 is In Progress under EMBERRS-001 retirement authority.
Stored `N/M` is left to the `pnpm aps:index` reconcile (ADR-053).

> **Exclusive module.** The wave orchestrator is the only plan writer. Parallel
> executors own isolated code/test workspaces and do not edit this module, the
> APS index, claims, or publication state.

## Purpose

Repair the 24 substantiated Clawpatch findings not already implemented by
SETCON-012, without turning scanner records into one work item each or using the
shared CIB backlog as feature-branch state.

## Source truth

- [Clawpatch open-findings review](../../docs/reviews/2026-08-28-clawpatch-open-findings.md)
- Clawpatch persisted finding receipts and current-source anchors named there
- Accepted ADRs and owning package contracts cited by each finding
- Current source at `4f84527c9e482f575a9417dd368c109fc24bd722`

## In scope

- Twelve coherent repair clusters covering findings 1 and 6–28 from the review.
- Focused RED/GREEN regression evidence for every behavioural repair.
- Independent verification and Council convergence before publication.
- Readme or operational documentation updates only where the owning contract
  changes or a selected finding is documentation-specific.

## Bounded first delivery

The approved first publication boundary is CLAWOPEN-001, -003, -004, -005, -006,
-008, and -012: 14 CLAWOPEN findings. SETCON-012 independently governs four
settings findings carried by the same candidate, producing an 18-of-28 combined
delivery without folding those settings repairs into this module.

CLAWOPEN-002 and CLAWOPEN-010 later merged via
[#4219](https://github.com/eddacraft/anvil-001/pull/4219). CLAWOPEN-007, -009,
and -011 remain outside that merged delivery: five findings not yet closed. The
later #4361 records approved designs for -007/-011 and carries their unmerged
implementation; -009 now follows the operator-approved retirement under
EMBERRS-001. Their status is not a defect in the bounded candidate and the
candidate must not claim that all 24 CLAWOPEN findings, or all 28 reviewed
findings, are repaired.

## Out of scope

- Reopening the four SETCON-012 settings repairs covered by findings 2–5.
- The triage report's one uncertain Ember concurrency allegation.
- The historical lifetime-open Clawpatch queue outside the 28 reviewed records.
- Shared-CIB bookkeeping, release claims, or administrator-policy overrides.
- Merge or release authority.

## Work Items

### CLAWOPEN-001: Freeze process-wide flag authority

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P1
- **Risk:** standard
- **Intent:** Consumers cannot mutate any validated process-wide flag inventory
  or alter later catalogue decisions through import order.
- **Expected Outcome:** The manifest, group, audience, and environment
  inventories are recursively frozen before derived maps are constructed; public
  accessors expose compatible deep-readonly contracts; attempted nested mutation
  leaves each accessor and `flagByKey` result unchanged.
- **Files:** `packages/anvil/flags-catalogue/src/manifest.ts`,
  `packages/anvil/flags-catalogue/src/catalogue.ts`, package tests and contract
  documentation if its public type changes.
- **Finding ID:** `fnd_sig-feat-library-4b653635ed-942f_02d471ad28`
- **Validation:** `pnpm --dir packages/anvil/flags-catalogue test -- --run`;
  `pnpm --dir packages/anvil/flags-catalogue typecheck`;
  `pnpm --dir packages/anvil/flags-catalogue build`
- **Decision:** ready

### CLAWOPEN-002: Restore suspended-account approval

- **Status:** Merged 2026-08-30 via PR #4219. Ancestor of `origin/main`
  (`6e728c0f7`).
- **Priority:** P1
- **Risk:** high
- **Intent:** An operator can reactivate a previously approved suspended account
  without weakening first-approval atomicity, scope checks, or audit evidence.
- **Expected Outcome:** First approval, duplicate active approval, suspended
  reactivation, and concurrent reactivation have distinct deterministic
  outcomes; only a successful transition produces grant side effects.
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/__tests__/admin.test.ts`
- **Finding ID:** `fnd_sig-feat-route-8799ede6c4-dd3891_c05bd63a20`
- **Validation:**
  `pnpm --dir apps/anvil-api exec vitest run src/__tests__/admin.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Decision:** ready

### CLAWOPEN-003: Recover GitHub device sessions and verified identity

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P1
- **Risk:** high
- **Intent:** Durable minted sessions replay without live OAuth credentials, and
  first-link identity accepts a verified secondary email when the primary is
  unverified.
- **Expected Outcome:** Polling checks and returns an unexpired stored minted
  session before credential lookup or upstream work; credentials remain
  mandatory immediately before exchange; canonical email selection prefers a
  verified primary then falls back to another verified email while preserving
  every verified address for linking.
- **Files:** `apps/anvil-api/src/routes/auth-github-device.ts`,
  `apps/anvil-api/src/lib/github-user.ts`,
  `apps/anvil-api/src/__tests__/auth-github-device.test.ts`,
  `apps/anvil-api/src/__tests__/auth-github.test.ts`,
  `apps/anvil-api/src/__tests__/github-user.test.ts`
- **Finding IDs:** `fnd_sig-feat-route-dad030c9a3-d923ad_09016e1842`,
  `fnd_sig-feat-service-b6b9358432-46e3_bbe96abfba`
- **Validation:**
  `pnpm --dir apps/anvil-api exec vitest run src/__tests__/auth-github-device.test.ts src/__tests__/auth-github.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Decision:** ready

### CLAWOPEN-004: Keep persisted waitlist success truthful

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P1
- **Risk:** high
- **Intent:** A successfully persisted waitlist signup is never reported to the
  user as failed solely because the admin notification failed.
- **Expected Outcome:** Notification is best-effort after persistence, the route
  returns success or accepted, and the internal failure remains observable.
- **Files:** `apps/anvil-api/src/routes/waitlist.ts`,
  `apps/anvil-api/src/__tests__/waitlist.test.ts`
- **Finding ID:** `fnd_sig-feat-route-19b5dcc053-32bace_ecd0c8bd99`
- **Validation:**
  `pnpm --dir apps/anvil-api exec vitest run src/__tests__/waitlist.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Decision:** ready

### CLAWOPEN-005: Pin security-sensitive route decisions

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P1
- **Risk:** high
- **Intent:** Route-level tests fail when OAuth callback or early-access
  authorisation decisions regress.
- **Expected Outcome:** Focused suites cover every decision enumerated in
  findings 10 and 11.
- **Files:** docs-shell callback tests, website early-access install tests, and
  their existing route sources only if a regression exposes contract drift.
- **Finding IDs:** `fnd_sig-feat-route-c9bbbefdb4-3d5549_d877699857`,
  `fnd_sig-feat-route-0fc07f4172-f0ced0_6460acabcb`
- **Validation:** focused Vitest suites for both routes; affected project
  typechecks
- **Decision:** ready

### CLAWOPEN-006: Reject false-valid boundary data

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P2
- **Risk:** standard
- **Intent:** Invalid epoch inputs and whitespace-only diagram accessibility
  metadata fail validation instead of becoming plausible data.
- **Expected Outcome:** Epoch conversion accepts finite numbers or non-empty
  numeric strings only; diagram titles and descriptions must remain non-empty
  after trimming.
- **Files:** `apps/anvil-api/src/lib/account-activity-metrics.ts`, its focused
  tests, `scripts/docs/lib/public-diagrams.mjs`, and
  `scripts/docs/check-public-diagrams.test.mjs`
- **Finding IDs:** `fnd_sig-feat-library-83f17ec600-78d2_38bd906098`,
  `fnd_sig-feat-library-5ed95bd031-ebb5_118ed07435`
- **Validation:** focused API metrics Vitest;
  `node --test scripts/docs/check-public-diagrams.test.mjs`;
  `pnpm docs:public:diagrams`
- **Decision:** ready

### CLAWOPEN-007: Make generated docs durable

- **Status:** Merged 2026-09-05 via PR #4361
- **Closeout evidence (2026-09-09):**
  [#4361](https://github.com/eddacraft/anvil-001/pull/4361) merged to `main` at
  `de222cbc` on 2026-09-05, carrying `scripts/docs/lib/atomic-output-batch.mjs`
  and routing both generators through it. Re-verified on `main` at `82db0626`:
  `node --test scripts/docs/atomic-output-batch.test.mjs` 6/6,
  `node --test scripts/docs/generator-atomic-output.test.mjs` 7/7,
  `pnpm docs:public:check` 0 errors across 98 files, and
  `pnpm docs:catalogue:check` current. The remaining `pnpm test:docs-check` and
  `pnpm docs:check` legs are covered by the merge's own CI rather than re-run
  here.
- **Priority:** P2
- **Risk:** standard
- **Intent:** Documentation generation cannot destroy the prior valid outputs.
- **Expected Outcome:** Both generators prepare every requested output in unique
  same-directory temporary files before replacement and restore the complete
  prior batch on an injected write or rename failure. Public-reference help
  snapshots join the same transaction when `--update-help-snapshots` is used;
  `--check --update-help-snapshots` is rejected without writing.
- **Files:** `scripts/docs/generate-anvil-public-reference.mjs`,
  `scripts/docs/generate-product-catalogue.mjs`, and focused generator tests
- **Finding ID:** `fnd_sig-feat-library-49c5c2a728-0640_c321ad6f8f`
- **Validation:** `node --test scripts/docs/atomic-output-batch.test.mjs`;
  `pnpm test:docs-check`; `pnpm docs:public:check`;
  `pnpm docs:catalogue:check`; `pnpm docs:check`
- **Decision:** closed — the whole-batch rollback and `--update-help-snapshots`
  boundary design merged via #4361.

### CLAWOPEN-008: Make evaluation evidence non-vacuous

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P2
- **Risk:** standard
- **Intent:** Default test commands exercise real extraction, calibrated benign
  cases, structured JSON output, and panic-safe temporary-file cleanup.
- **Expected Outcome:** The four finding-specific failure modes are each
  reproduced by a focused test that then passes under the normal crate command.
- **Files:** `crates/anvil-checks/tests/conformance_evaluate.rs`,
  `crates/anvil-checks/tests/secret_calibration.rs`,
  `crates/anvil-checks/tests/corpus/secret/manifest.json`,
  `crates/anvil-cli/tests/ast_followup.rs`,
  `crates/anvil-checks/tests/secret_file_coverage.rs`
- **Finding IDs:** `fnd_sig-feat-test-suite-3ee4f472b1-6_9d9d7dc190`,
  `fnd_sig-feat-test-suite-77ddfbae71-4_f4106d3c1b`,
  `fnd_sig-feat-test-suite-794320ab38-7_cc53786ee0`,
  `fnd_sig-feat-test-suite-f53827d8bc-e_ddafcd834c`
- **Validation:** the four focused Rust integration targets;
  `cargo test -p eddacraft-anvil-checks`;
  `cargo test -p eddacraft-anvil --no-fail-fast`
- **Decision:** ready — the owning CONF-004, SDT-002, GTAO-003, and SDT-006 pull
  requests are merged; a fresh branch and worktree check found no live change in
  the five owned files.

### CLAWOPEN-009: Retire unsupported JS runtime outcomes

- **Status:** In Progress
- **Priority:** P2
- **Risk:** high
- **Intent:** Remove unsupported JS execution paths instead of repairing retired
  implementations.
- **Expected Outcome:** EMBERRS-001 removes JS watch/queue and TypeScript Ember
  implementations from source, builds and package exports; the Rust Ember reader
  is default-off. User data and the live API/docs flag resolver are preserved.
  The publication-failure regression is carried into EMBERRS-004 as Rust
  acceptance evidence, not claimed fixed in historical JS.
- **Files:** `packages/anvil/runtime`, `packages/edda-stack`,
  `crates/anvil-cli/src/commands/ember.rs`, `flags/manifest.json`
- **Finding IDs:** `fnd_sig-feat-library-5982411632-4fab_42a7488950`,
  `fnd_sig-feat-library-d492a0dec3-66c5_7f981bd76a`,
  `fnd_sig-feat-library-7c0d8094f1-a367_a4317f40f7`
- **Validation:** `node --test scripts/ci/legacy-runtime-retirement.test.mjs`;
  `cargo test -p eddacraft-anvil --test ember_gate`; affected package tests.
- **Decision:** Operator approved retirement and Rust migration planning on
  2026-09-05. [EMBERRS-001](ember-rust-migration.aps.md) owns this delivery and
  claim #4398. Close these receipts as retired only after merge/verification;
  migration planning alone is not a repair or retirement completion claim.

### CLAWOPEN-010: Pin docs-shell behaviour and caching

- **Status:** Merged 2026-08-30 via PR #4219. Ancestor of `origin/main`
  (`6e728c0f7`).
- **Priority:** P3
- **Risk:** standard
- **Intent:** User-facing auth recovery and landing-page destinations remain
  covered while the static landing route avoids needless request-time rendering.
- **Expected Outcome:** Focused component tests pin recognised/fallback auth
  errors, pending recovery links, and root destinations; the production build
  classifies the root route as static or deliberately revalidated.
- **Files:** `apps/docs-shell/app/auth/error/page.tsx`,
  `apps/docs-shell/app/auth/pending/page.tsx`, `apps/docs-shell/app/page.tsx`,
  `apps/docs-shell/app/auth/error/page.test.ts`,
  `apps/docs-shell/app/auth/pending/page.test.ts`,
  `apps/docs-shell/app/page.test.ts`, `apps/docs-shell/vitest.config.ts`
- **Finding IDs:** `fnd_sig-feat-route-9eb65fea2f-a2ad37_77eb4d489d`,
  `fnd_sig-feat-route-ea68cde701-4d56a1_c1562065ab`,
  `fnd_sig-feat-route-f44022f02c-73c870_78633640ed`,
  `fnd_sig-feat-route-f44022f02c-6068aa_9490310487`
- **Validation:** focused docs-shell Vitest; docs-shell typecheck and production
  build
- **Decision:** ready

### CLAWOPEN-011: Prove OTP attempt caps against PostgreSQL

- **Status:** Complete 2026-09-09 — hosted Neon proof green on `main`
- **Closeout evidence (2026-09-09):** Hosted provisioning is configured and the
  live proof passes end to end. Run
  [34305126477](https://github.com/eddacraft/anvil-001/actions/runs/34305126477)
  (`workflow_dispatch` on `main` at `82db0626`) created the ephemeral branch
  `ci-test-clawopen-011-34305126477-1` (`br-odd-flower-axubaaml`, two-hour
  expiry) in the `anvil-api-test` project, passed
  `apps/anvil-api/src/__tests__/auth-otp.neon.test.ts` — "allows exactly three
  claims after more than three callers visibly contend on the row lock" — and
  deleted the branch. The 2026-09-05 `NEON_API_KEY is required` failure
  (run 33893142132) was cleared by
  [#4473](https://github.com/eddacraft/anvil-001/pull/4473), which pointed the
  workflow at the configured `NEON_TEST_API_KEY` secret and
  `NEON_TEST_PROJECT_ID` variable. Supporting local evidence on `82db0626`:
  focused API suites 44/44 (`auth-otp`, `neon-otp-contention`,
  `neon-test-database-safety`, `neon-client-retry`) and
  `pnpm --dir apps/anvil-api typecheck` clean.
- **Priority:** P1
- **Risk:** high
- **Intent:** The OTP attempt cap is verified against the real database
  statement rather than a JavaScript mock that assumes atomic behaviour.
- **Expected Outcome:** A dedicated non-production Neon project supplies an
  ephemeral branch per CI run. Before creating it, the repository-owned
  control-plane helper resolves the configured project through Neon and
  requires the exact `anvil-api-test` identity; generated connection
  credentials are masked before any step output is emitted, and partial
  creation retains exact cleanup identity. The live proof holds a
  `FOR UPDATE` lock on one active OTP, starts more than the maximum attempts
  through the production query, and does not release the barrier until
  `pg_stat_activity` reports at least four matching OTP updates active and
  waiting on a lock. Returned claims are exactly attempts 1–3, stored attempts
  remain exactly three, and cleanup runs even after test failure.
- **Files:** `apps/anvil-api/src/routes/auth-otp.ts`, its query/schema
  dependencies, a Neon-backed integration test, the dedicated CI workflow,
  and the owning test/runbook documentation.
- **Finding ID:** `fnd_sig-feat-route-c6c95ee31e-9b089f_43160b2454`
- **Validation:** the repository's PostgreSQL-backed API integration command;
  focused API tests and typecheck
- **Decision:** closed — the disposable PostgreSQL harness merged via #4361 and
  the hosted real-database verification is green on `main`.

### CLAWOPEN-012: Complete operational API documentation

- **Status:** Merged 2026-08-30 via PR #4216. Ancestor of `origin/main`
  (`965a9e7f4`).
- **Priority:** P2
- **Risk:** standard
- **Intent:** API operators can discover every required GitHub credential and
  the admin fleet endpoint from the owning README.
- **Expected Outcome:** The README documents hosted and CLI credential names,
  purpose, provider, route ownership, and the fleet endpoint's authentication
  and response purpose without values; the environment example is reconciled
  with that authority.
- **Files:** `apps/anvil-api/README.md`, `.env.example` if it is confirmed as
  the matching operator surface
- **Finding IDs:** `fnd_sig-feat-route-24c7d2a330-771f31_1ce2eab8d0`,
  `fnd_sig-feat-route-25b003a83f-6b469c_711e4b12d8`
- **Validation:** credential-helper and deployment cross-check; admin fleet
  route inventory cross-check; `pnpm docs:check`
- **Decision:** ready

## Sequencing

1. Run CLAWOPEN-001, -002, and -003 as the first independent executor wave when
   isolated workspace capacity is available.
2. Run CLAWOPEN-004 and -005 next; the API-auth work from -002/-003 must land
   before rebasing any overlapping API test helpers.
3. Run Ready clusters CLAWOPEN-006, -010, and -012 by owning package.
4. Complete review and hosted Neon verification for CLAWOPEN-007/-011 on #4361;
   its branch records design approval on 2026-08-31. Complete CLAWOPEN-009
   retirement through EMBERRS-001.
5. For a bounded publication, re-run the acceptance matrix for every finding
   included in that candidate, independently verify the exact head, and obtain
   Council convergence. Keep excluded clusters and their counts explicit.
6. Before closing the full wave, re-run the complete 24-finding acceptance
   matrix, independent verification, and Council.

## Wave validation

- For the approved first delivery, verify all 14 findings mapped to
  CLAWOPEN-001, -003, -004, -005, -006, -008, and -012.
- `pnpm validate:changed`
- `pnpm format:check`
- `pnpm docs:check`
- Focused commands named by each completed work item
- Independent `verify-loop` against every mapped finding
- Council convergence on the exact final head
