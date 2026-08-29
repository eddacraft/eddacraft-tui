<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if work items exist and status is Ready. -->

# Clawpatch Open-Findings Repair Wave

| ID       | Owner | Priority | Status      | Progress |
| -------- | ----- | -------- | ----------- | -------- |
| CLAWOPEN | —     | P1       | In Progress | 0/12     |

**Last reviewed:** 2026-08-29 — operator selected the 24 findings that remain
after SETCON-012 from the
[2026-08-28 complete-store triage](../../docs/reviews/2026-08-28-clawpatch-open-findings.md)
for a bounded repair wave. CLAWOPEN-001 is the first executable slice.
CLAWOPEN-007, -008, -009, and -011 retain explicit design or dependency
checkpoints; the remaining clusters are Ready under the acceptance boundaries
below.

> **Exclusive module.** The wave orchestrator is the only plan writer.
> Parallel executors own isolated code/test workspaces and do not edit this
> module, the APS index, claims, or publication state.

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

- Ten coherent repair clusters covering findings 1 and 6–28 from the review.
- Focused RED/GREEN regression evidence for every behavioural repair.
- Independent verification and Council convergence before publication.
- Readme or operational documentation updates only where the owning contract
  changes or a selected finding is documentation-specific.

## Out of scope

- Reopening the four SETCON-012 settings repairs covered by findings 2–5.
- The triage report's one uncertain Ember concurrency allegation.
- The historical lifetime-open Clawpatch queue outside the 28 reviewed records.
- Shared-CIB bookkeeping, release claims, or administrator-policy overrides.
- Merge or release authority.

## Work Items

### CLAWOPEN-001: Freeze process-wide flag authority

- **Status:** Blocked
- **Priority:** P1
- **Risk:** standard
- **Intent:** Consumers cannot mutate any validated process-wide flag inventory
  or alter later catalogue decisions through import order.
- **Expected Outcome:** The manifest, group, audience, and environment
  inventories are recursively frozen before derived maps are constructed;
  public accessors expose compatible deep-readonly contracts; attempted nested
  mutation leaves each accessor and `flagByKey` result unchanged.
- **Files:** `packages/anvil/flags-catalogue/src/manifest.ts`,
  `packages/anvil/flags-catalogue/src/catalogue.ts`,
  package tests and contract documentation if its public type changes.
- **Finding ID:** `fnd_sig-feat-library-4b653635ed-942f_02d471ad28`
- **Validation:** `pnpm --dir packages/anvil/flags-catalogue test -- --run`;
  `pnpm --dir packages/anvil/flags-catalogue typecheck`;
  `pnpm --dir packages/anvil/flags-catalogue build`
- **Decision:** ready

### CLAWOPEN-002: Restore suspended-account approval

- **Status:** Blocked
- **Priority:** P1
- **Risk:** high
- **Intent:** An operator can reactivate a previously approved suspended
  account without weakening first-approval atomicity, scope checks, or audit
  evidence.
- **Expected Outcome:** First approval, duplicate active approval, suspended
  reactivation, and concurrent reactivation have distinct deterministic
  outcomes; only a successful transition produces grant side effects.
- **Files:** `apps/anvil-api/src/routes/admin.ts`,
  `apps/anvil-api/src/__tests__/admin.test.ts`
- **Finding ID:** `fnd_sig-feat-route-8799ede6c4-dd3891_c05bd63a20`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run src/__tests__/admin.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Blocker:** The write gate requires exact operator approval for changes to
  the production admin transaction that updates account status and creates
  token and audit rows.
- **Decision:** ready

### CLAWOPEN-003: Recover GitHub device sessions and verified identity

- **Status:** In Progress
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
  `apps/anvil-api/src/__tests__/auth-github.test.ts`
- **Finding IDs:** `fnd_sig-feat-route-dad030c9a3-d923ad_09016e1842`,
  `fnd_sig-feat-service-b6b9358432-46e3_bbe96abfba`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run
  src/__tests__/auth-github-device.test.ts src/__tests__/auth-github.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Blocker:** The pre-write validator rejected the first RED test patch for a
  context mismatch and marked the submission non-retriable. No source or test
  change was applied.
- **Decision:** ready

### CLAWOPEN-004: Keep persisted waitlist success truthful

- **Status:** In Progress
- **Priority:** P1
- **Risk:** high
- **Intent:** A successfully persisted waitlist signup is never reported to the
  user as failed solely because the admin notification failed.
- **Expected Outcome:** Notification is best-effort after persistence, the route
  returns success or accepted, and the internal failure remains observable.
- **Files:** `apps/anvil-api/src/routes/waitlist.ts`,
  `apps/anvil-api/src/__tests__/waitlist.test.ts`
- **Finding ID:** `fnd_sig-feat-route-19b5dcc053-32bace_ecd0c8bd99`
- **Validation:** `pnpm --dir apps/anvil-api exec vitest run src/__tests__/waitlist.test.ts`;
  `pnpm --dir apps/anvil-api typecheck`
- **Decision:** ready

### CLAWOPEN-005: Pin security-sensitive route decisions

- **Status:** Ready
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

- **Status:** Ready
- **Priority:** P2
- **Risk:** standard
- **Intent:** Invalid epoch inputs and whitespace-only diagram accessibility
  metadata fail validation instead of becoming plausible data.
- **Expected Outcome:** Epoch conversion accepts finite numbers or non-empty
  numeric strings only; diagram titles and descriptions must remain non-empty
  after trimming.
- **Files:** `apps/anvil-api/src/lib/account-activity-metrics.ts`,
  its focused tests, `scripts/docs/lib/public-diagrams.mjs`, and
  `scripts/docs/check-public-diagrams.test.mjs`
- **Finding IDs:** `fnd_sig-feat-library-83f17ec600-78d2_38bd906098`,
  `fnd_sig-feat-library-5ed95bd031-ebb5_118ed07435`
- **Validation:** focused API metrics Vitest; `node --test
  scripts/docs/check-public-diagrams.test.mjs`; `pnpm docs:public:diagrams`
- **Decision:** ready

### CLAWOPEN-007: Make generated docs durable

- **Status:** Proposed
- **Priority:** P2
- **Risk:** standard
- **Intent:** Documentation generation cannot destroy the prior valid outputs.
- **Expected Outcome:** Both generators prepare unique same-directory temporary
  files before replacement and preserve prior outputs on injected write or
  rename failure.
- **Files:** `scripts/docs/generate-anvil-public-reference.mjs`,
  `scripts/docs/generate-product-catalogue.mjs`, and focused generator tests
- **Finding ID:** `fnd_sig-feat-library-49c5c2a728-0640_c321ad6f8f`
- **Validation:** injected-failure generator tests; `pnpm docs:public:check`;
  `pnpm docs:catalogue:check`; `pnpm docs:check`
- **Decision:** needs-design — define whole-batch rollback semantics and the
  `--update-help-snapshots` boundary before implementation

### CLAWOPEN-008: Make evaluation evidence non-vacuous

- **Status:** Proposed
- **Priority:** P2
- **Risk:** standard
- **Intent:** Default test commands exercise real extraction, calibrated benign
  cases, structured JSON output, and panic-safe temporary-file cleanup.
- **Expected Outcome:** The four finding-specific failure modes are each
  reproduced by a focused test that then passes under the normal crate command.
- **Files:** `crates/anvil-checks/tests/conformance_evaluate.rs`,
  `crates/anvil-checks/tests/secret_calibration.rs`,
  `crates/anvil-cli/tests/ast_followup.rs`,
  `crates/anvil-checks/tests/secret_file_coverage.rs`
- **Finding IDs:** `fnd_sig-feat-test-suite-3ee4f472b1-6_9d9d7dc190`,
  `fnd_sig-feat-test-suite-77ddfbae71-4_f4106d3c1b`,
  `fnd_sig-feat-test-suite-794320ab38-7_cc53786ee0`,
  `fnd_sig-feat-test-suite-f53827d8bc-e_ddafcd834c`
- **Validation:** the four focused Rust integration targets; `cargo test -p
  eddacraft-anvil-checks`; `cargo test -p eddacraft-anvil --no-fail-fast`
- **Decision:** needs-dependency-reconciliation — confirm CONF-004 and SDT-002
  ownership before changing their active test surfaces

### CLAWOPEN-009: Resolve supported legacy runtime outcomes

- **Status:** Proposed
- **Priority:** P2
- **Risk:** high
- **Intent:** Supported legacy runtime paths have explicit selector,
  persistence/publication, and queued-lock fairness contracts.
- **Expected Outcome:** `since` combinations are either implemented or
  rejected explicitly; Ember retry cannot duplicate a proposal after
  publication failure; queued lock waiters cannot be bypassed.
- **Files:** `packages/anvil/runtime/src/watch/git-status.ts`,
  `packages/edda-stack/src/ember/candidate-service.ts`,
  `packages/anvil/runtime/src/concurrency/queue-manager.ts`, and focused tests
- **Finding IDs:** `fnd_sig-feat-library-5982411632-4fab_42a7488950`,
  `fnd_sig-feat-library-d492a0dec3-66c5_7f981bd76a`,
  `fnd_sig-feat-library-7c0d8094f1-a367_a4317f40f7`
- **Validation:** affected runtime and edda-stack test/typecheck commands
- **Decision:** needs-design — select the combined-selector and Ember
  publication contracts before implementation

### CLAWOPEN-010: Pin docs-shell behaviour and caching

- **Status:** Ready
- **Priority:** P3
- **Risk:** standard
- **Intent:** User-facing auth recovery and landing-page destinations remain
  covered while the static landing route avoids needless request-time rendering.
- **Expected Outcome:** Focused component tests pin recognised/fallback auth
  errors, pending recovery links, and root destinations; the production build
  classifies the root route as static or deliberately revalidated.
- **Files:** `apps/docs-shell/app/auth/error/page.tsx`,
  `apps/docs-shell/app/auth/pending/page.tsx`,
  `apps/docs-shell/app/page.tsx`, and focused tests
- **Finding IDs:** `fnd_sig-feat-route-9eb65fea2f-a2ad37_77eb4d489d`,
  `fnd_sig-feat-route-ea68cde701-4d56a1_c1562065ab`,
  `fnd_sig-feat-route-f44022f02c-73c870_78633640ed`,
  `fnd_sig-feat-route-f44022f02c-6068aa_9490310487`
- **Validation:** focused docs-shell Vitest; docs-shell typecheck and production
  build
- **Decision:** ready

### CLAWOPEN-011: Prove OTP attempt caps against PostgreSQL

- **Status:** Proposed
- **Priority:** P1
- **Risk:** high
- **Intent:** The OTP attempt cap is verified against the real database
  statement rather than a JavaScript mock that assumes atomic behaviour.
- **Expected Outcome:** More than the maximum attempts run concurrently against
  one active OTP; returned claims and stored attempts never exceed the cap.
- **Files:** `apps/anvil-api/src/routes/auth-otp.ts`, its query/schema
  dependencies, and a PostgreSQL-backed integration test.
- **Finding ID:** `fnd_sig-feat-route-c6c95ee31e-9b089f_43160b2454`
- **Validation:** the repository's PostgreSQL-backed API integration command;
  focused API tests and typecheck
- **Decision:** needs-design — establish the disposable PostgreSQL harness
  owned by TEXT before implementation

### CLAWOPEN-012: Complete operational API documentation

- **Status:** Ready
- **Priority:** P2
- **Risk:** standard
- **Intent:** API operators can discover every required GitHub credential and
  the admin fleet endpoint from the owning README.
- **Expected Outcome:** The README documents hosted and CLI credential names,
  purpose, provider, route ownership, and the fleet endpoint's authentication
  and response purpose without values; the environment example is reconciled
  with that authority.
- **Files:** `apps/anvil-api/README.md`, `.env.example` if it is confirmed
  as the matching operator surface
- **Finding IDs:** `fnd_sig-feat-route-24c7d2a330-771f31_1ce2eab8d0`,
  `fnd_sig-feat-route-25b003a83f-6b469c_711e4b12d8`
- **Validation:** credential-helper and deployment cross-check; admin fleet
  route inventory cross-check; `pnpm docs:check`
- **Decision:** ready

## Sequencing

1. Run CLAWOPEN-001, -002, and -003 as the first independent executor wave
   when isolated workspace capacity is available.
2. Run CLAWOPEN-004 and -005 next; the API-auth work from -002/-003 must land
   before rebasing any overlapping API test helpers.
3. Run Ready clusters CLAWOPEN-006, -010, and -012 by owning package.
4. Reconcile CLAWOPEN-008 dependencies and pass CLAWOPEN-007, -009, and -011
   through their design membranes before implementation.
5. Re-run the full 24-finding acceptance matrix, independent verification, and
   Council before publication.

## Wave validation

- `pnpm validate:changed`
- `pnpm format:check`
- `pnpm docs:check`
- Focused commands named by each completed work item
- Independent `verify-loop` against every mapped finding
- Council convergence on the exact final head
