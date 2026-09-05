# Clawpatch open findings — 2026-08-28

| Type  | Authority | Owner | Status | Freshness                                                                                                                                                                                    |
| ----- | --------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | CIB   | Live   | Reviewed 2026-08-29 against seven Clawpatch runs, persisted finding receipts, governing contracts, and current source under `apps/`, `crates/`, `packages/`, and `scripts/docs` at 4f84527c9 |

| Upstream                                                                                                                                                                                                                                          | Downstream                                                                                                |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Clawpatch finding records from the seven runs listed below; [documentation governance](../guides/documentation-governance.md); [CLAWSCAN archive](../../plans/archive/modules/clawpatch-recent-scan-repair-wave.aps.md); current source and tests | Operator-selected APS repair wave or opportunistic fixes; this report does not create execution authority |

**Source paths reviewed:**

- `apps/anvil-api/src`
- `apps/docs-shell/app`
- `apps/website/app/api/early-access/install`
- `crates/anvil-checks/tests`
- `crates/anvil-cli/tests`
- `crates/anvil-settings/src`
- `packages/anvil/flags-catalogue/src`
- `packages/anvil/runtime/src`
- `packages/edda-stack/src/ember`
- `scripts/docs`
- `apps/anvil-api/README.md`
- `packages/anvil/runtime/README.md`

## Decision

Keep 28 findings open. They are substantiated against current source, but they
are not one undifferentiated release gate:

- 11 are P1 recommendations: contract, customer-state, authority, or
  security-sensitive coverage work.
- 9 are P2 recommendations: bounded correctness, data-integrity, operational
  documentation, or confidence work.
- 8 are P3 recommendations: advisory coverage, performance, or hygiene work.
- There is no P0 emergency in this set.
- Do not create one APS or CIB item per finding. Promote coherent repair
  clusters only after an operator chooses scope.

This document is review evidence, not a second backlog. APS modules and their
work items remain the only execution authority.

## Corpus and scope

The source records came from these completed runs:

- 20260819T164749-cb786b
- 20260821T071150-2de940
- 20260822T164542-03bbaf
- 20260826T130534-89c857
- 20260827T161339-1e3547
- 20260828T061029-a16dca
- 20260828T140205-f2e7d5

The complete persisted store was inventoried before selecting this tail. The
triage session dispositioned 60 records: 28 open, 21 false-positive, 6 fixed, 4
wont-fix, and 1 uncertain. This report covers only the 28 open records. The
uncertain Ember candidate-cap concurrency claim remains outside this report
until a two-session real-store reproduction exists.

Snapshot after triage:

| Metric                      |                                    Value |
| --------------------------- | ---------------------------------------: |
| Persisted findings          |                                      986 |
| Open findings               |                                      510 |
| Open high-severity findings |                                        2 |
| Active locks                |                                        0 |
| Lock files                  |                                        0 |
| Current-source anchor       | 4f84527c9e482f575a9417dd368c109fc24bd722 |

The lifetime open count includes previously reviewed historical advisories. It
must not be read as 510 newly confirmed defects.

## Method

1. Inventory the full finding, run, and report stores rather than treating the
   latest report as the queue.
2. Compare each selected allegation with current source, tests, governing APS
   and ADR contracts, reachability, and product maturity.
3. Deduplicate regenerated findings against accepted canonical records.
4. Keep static concurrency claims uncertain unless direct source sequencing or
   runtime evidence proves them.
5. Recheck the eleven commits between the original triage anchor and 00c2e6e61.
   None repaired any of these 28 findings. The two latest commits changed Rust
   CLI, TUI, and edda-memory documentation outside the cited surfaces. One cited
   generator changed its formatter launcher earlier in the range, but its final
   output write remains direct.
6. Recheck the five later commits through 4f84527c9. The only changed file under
   the report's broad source-path inventory was an unrelated generated CLI-help
   fixture; none of the 28 cited implementation, test, README, or generator
   files changed.
7. Persist an evidence note on every selected finding and read every receipt
   back.

## Priority summary

| Priority | Cluster                             | Findings | Recommended handling                                                                 |
| -------- | ----------------------------------- | -------: | ------------------------------------------------------------------------------------ |
| P1       | Settings truth                      |        4 | One settings repair wave before governed mutation surfaces depend on these contracts |
| P1       | Account and identity recovery       |        3 | One API auth repair wave with focused negative-path tests                            |
| P1       | Process-wide flag authority         |        1 | Small catalogue hardening change                                                     |
| P1       | Waitlist response truth             |        1 | Small API correctness change                                                         |
| P1       | Security-sensitive route coverage   |        2 | One focused website/docs-auth test wave                                              |
| P2       | Boundary and data integrity         |        3 | Small independent fixes                                                              |
| P2       | Concurrency and evaluation evidence |        3 | Group by owning test surface, not by scanner record                                  |
| P2       | Legacy and draft runtime contracts  |        2 | Fix opportunistically or explicitly narrow the contract                              |
| P2       | Operational API documentation       |        1 | Update the owning README                                                             |
| P3       | Docs-shell coverage and performance |        4 | One docs-shell smoke-test/performance batch                                          |
| P3       | Fleet endpoint documentation        |        1 | Fold into the API README update                                                      |
| P3       | Test harness hygiene                |        2 | Opportunistic test hardening                                                         |
| P3       | Winding-down JS runtime fairness    |        1 | Fix only if the path remains supported                                               |

## P1 — contract, authority, and customer-state correctness

### 1. Mutable manifest accessors can corrupt the flag authority

- **Finding:** fnd_sig-feat-library-4b653635ed-942f_02d471ad28
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [manifest accessors](../../packages/anvil/flags-catalogue/src/manifest.ts),
  [catalogue accessor precedent](../../packages/anvil/flags-catalogue/src/catalogue.ts),
  and the [package contract](../../packages/anvil/flags-catalogue/README.md).
- **Assessment:** The manifest, group, audience, and environment accessors
  return module-singleton objects directly. A consumer can mutate nested values
  and change later accessor and flagByKey results. Catalogue decisions then
  depend on consumer import order rather than immutable source data.
- **Minimum repair:** Deep-freeze all four validated inventories, expose
  deep-readonly return types, and construct derived maps from the frozen values.
- **Validation:** Attempt nested mutation through each public accessor and prove
  later accessor and flagByKey results remain unchanged.

### 2. Resolver values are not checked against catalogue types

- **Finding:** fnd_sig-feat-library-90c6a94493-3f60_9ee60decc4
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [catalogue value types](../../crates/anvil-settings/src/types.rs),
  [resolver](../../crates/anvil-settings/src/resolver.rs), and
  [settings service](../../crates/anvil-settings/src/service.rs).
- **Assessment:** Resolution accepts JSON values without checking the
  catalogue's ValueType. The service can therefore emit a value that contradicts
  the setting definition while presenting the snapshot as internally consistent.
- **Minimum repair:** Validate every candidate value against its catalogue type
  before merge or replacement semantics run; return a deterministic typed
  failure.
- **Validation:** Cover boolean, string, enum, list, and map mismatches plus a
  valid value for each type.

### 3. RequireApproval constraints are a no-op

- **Finding:** fnd_sig-feat-library-90c6a94493-a47c_e469ff7b9a
- **Scanner classification:** high · confirmed-bug
- **Evidence:**
  [constraint evaluator](../../crates/anvil-settings/src/constraints.rs),
  [resolved setting shape](../../crates/anvil-settings/src/resolver.rs), and
  [SETCON-005](../../plans/archive/modules/settings-truth-contract.aps.md#setcon-005-policy-constraint-layer).
- **Assessment:** RequireApproval returns success without evaluating approval
  state, and ResolvedSetting carries no approval evidence. This is a genuine
  settings contract gap, but not an active shipped exploit because the governed
  mutation surface has not shipped.
- **Minimum repair:** Represent approval authority and evidence at the policy
  boundary, bind it to immutable bundle content, key and authority, then fail
  closed when the required approval is absent or invalid.
- **Validation:** Prove missing, wrong-authority, expired, and valid approvals;
  keep this a precondition for SETGOV mutation readiness.

### 4. Equivalent maps produce different revisions and classified digests

- **Finding:** fnd_sig-feat-library-90c6a94493-2dae_ac42adc07a
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [runtime digest](../../crates/anvil-settings/src/runtime_state.rs),
  [model revision](../../crates/anvil-settings/src/service.rs), and
  [ADR-132](../../plans/decisions/132-settings-truth-contract.md).
- **Assessment:** preserve-order JSON serialisation feeds both digest paths.
  Semantically equivalent maps with different insertion order therefore appear
  to be different settings state.
- **Minimum repair:** Canonicalise object keys recursively before hashing, while
  preserving list order and scalar values.
- **Validation:** Resolve equivalent nested maps in different insertion orders
  and assert identical revisions and classified digests.

### 5. A non-final Delete does not reset accumulated collection state

- **Finding:** fnd_sig-feat-library-90c6a94493-b99c_8ca9b53256
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [resolver Delete handling](../../crates/anvil-settings/src/resolver.rs).
- **Assessment:** Delete clears the current value only when it is the final
  candidate. In a sequence such as append, Delete, append, the pre-Delete
  collection survives and contaminates the later result.
- **Minimum repair:** Treat Delete as a reset point during candidate traversal,
  then apply later candidates to an empty state.
- **Validation:** Add list and map cases for append/union, Delete, then a later
  append/union or replacement.

### 6. Previously approved accounts cannot be reapproved

- **Finding:** fnd_sig-feat-route-8799ede6c4-dd3891_c05bd63a20
- **Scanner classification:** high · contract-mismatch
- **Evidence:** [admin approval route](../../apps/anvil-api/src/routes/admin.ts)
  and
  [SEC-007](../../plans/modules/security.aps.md#sec-007-atomic-token-revocation-hardening).
- **Assessment:** The approval claim requires approved_at to be null. An account
  approved once and later suspended therefore cannot use the documented admin
  approval surface to return to active status.
- **Minimum repair:** Separate first approval from account reactivation while
  preserving the atomic claim and audit requirements.
- **Validation:** Cover first approval, duplicate approval, suspended-account
  reactivation, and concurrent reactivation attempts.

### 7. Minted-session replay depends on live OAuth credentials

- **Finding:** fnd_sig-feat-route-dad030c9a3-d923ad_09016e1842
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [GitHub device route](../../apps/anvil-api/src/routes/auth-github-device.ts)
  and [ADR-066](../../plans/decisions/066-github-device-flow-cli-auth.md).
- **Assessment:** Credential lookup happens before session lookup and stored
  minted-session replay. A transient credential outage blocks recovery of a
  licence that is already durably minted and needs no upstream call.
- **Minimum repair:** Look up and replay a valid minted session before requiring
  credentials; require credentials only immediately before token exchange.
- **Validation:** Remove one OAuth credential, return a valid stored minted
  session, and assert a confirmed response with no fetch, claim, or mint call.

### 8. Verified secondary emails are blocked by an unverified primary

- **Finding:** fnd_sig-feat-service-b6b9358432-46e3_bbe96abfba
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [GitHub identity fetch](../../apps/anvil-api/src/lib/github-user.ts),
  [ADR-066](../../plans/decisions/066-github-device-flow-cli-auth.md), and
  [auth as-built](../architecture/auth-as-built.md).
- **Assessment:** The implementation throws unless the primary email is
  verified, although the first-link contract accepts any verified GitHub email.
  Users with an unverified primary and verified secondary are rejected before
  the valid secondary can be matched.
- **Minimum repair:** Prefer a verified primary as the canonical email, fall
  back to another verified email, retain all verified emails for linking, and
  fail only when none are verified.
- **Validation:** Cover verified primary, unverified primary plus verified
  secondary, and no verified email.

### 9. Admin-notification failure reports a persisted waitlist signup as failed

- **Finding:** fnd_sig-feat-route-19b5dcc053-32bace_ecd0c8bd99
- **Scanner classification:** medium · confirmed-bug
- **Evidence:** [waitlist route](../../apps/anvil-api/src/routes/waitlist.ts)
  and [waitlist tests](../../apps/anvil-api/src/__tests__/waitlist.test.ts).
- **Assessment:** The database upsert completes before the awaited admin
  notification. Notification failure reaches the outer error handler and returns
  500 even though the signup exists, encouraging retries and presenting a false
  failure to the user.
- **Minimum repair:** Make the admin notification best-effort after persistence
  or introduce a durable outbox; never return a failed-signup response after a
  successful commit.
- **Validation:** Make the admin notification reject and assert a success or
  accepted response while the signup remains persisted and the failure is
  observable internally.

### 10. OAuth callback decisions have no route-level regression coverage

- **Finding:** fnd_sig-feat-route-c9bbbefdb4-3d5549_d877699857
- **Scanner classification:** medium · test-gap
- **Evidence:**
  [callback route](../../apps/docs-shell/app/auth/callback/route.ts),
  [docs-auth design](../../plans/specs/2026-04-03-docs-auth-gating-design.md),
  and [Vitest configuration](../../apps/docs-shell/vitest.config.ts).
- **Assessment:** State decryption, nonce comparison, provider errors, BAUTH
  outcomes, redirects, and cookie issuance are security-sensitive observable
  branches. None is exercised at the route boundary.
- **Minimum repair:** Add one focused route suite with mocked state and BAUTH
  dependencies; production changes are not required.
- **Validation:** Cover success, invalid or missing state, nonce mismatch,
  provider denial, pending, BAUTH error, redirect validation, session-cookie
  attributes, and nonce clearing.

### 11. Early-access install tests omit the authorisation decision

- **Finding:** fnd_sig-feat-route-0fc07f4172-f0ced0_6460acabcb
- **Scanner classification:** medium · test-gap
- **Evidence:**
  [install route](../../apps/website/app/api/early-access/install/route.ts) and
  [current tests](../../apps/website/app/api/early-access/install/route.test.ts).
- **Assessment:** Tests cover upstream timeouts but not malformed input,
  upstream status mapping, malformed JSON, or the valid-and-isEdict predicate
  that gates distribution of the install command.
- **Minimum repair:** Add table-driven route tests; no production change is
  required unless a case exposes drift.
- **Validation:** Cover missing input, upstream 401 and 5xx, malformed JSON,
  every false or missing valid/isEdict combination, and the both-true success
  response.

## P2 — bounded correctness, integrity, and confidence

### 12. Invalid epoch values are coerced to 1970

- **Finding:** fnd_sig-feat-library-83f17ec600-78d2_38bd906098
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [account activity mapping](../../apps/anvil-api/src/lib/account-activity-metrics.ts)
  and
  [tests](../../apps/anvil-api/src/__tests__/account-activity-metrics.test.ts).
- **Assessment:** Number coercion turns null, false, empty strings, and
  whitespace into zero. Malformed database values therefore become a plausible
  1970 timestamp and can distort activity windows and quiet-account cohorts.
- **Minimum repair:** Accept finite numbers or non-empty numeric strings only;
  reject null, booleans, and blank strings before conversion.
- **Validation:** Add null, false, empty, whitespace, non-numeric, and valid
  numeric cases.

### 13. Generated documentation is replaced non-atomically

- **Finding:** fnd_sig-feat-library-49c5c2a728-0640_c321ad6f8f
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [public-reference generator](../../scripts/docs/generate-anvil-public-reference.mjs)
  and
  [product-catalogue generator](../../scripts/docs/generate-product-catalogue.mjs).
- **Assessment:** Both generators overwrite committed output directly. An
  interruption or partial write can destroy a previously valid file; the
  multi-output reference generator can also leave a mixed generation.
- **Minimum repair:** Prepare unique temporary files in each destination
  directory, close and sync them as appropriate, then rename them over the
  destinations. Prepare the reference set before replacing any member.
- **Validation:** Inject final-write and rename failures and prove the previous
  outputs remain byte-identical.

### 14. Whitespace-only diagram accessibility metadata is accepted

- **Finding:** fnd_sig-feat-library-5ed95bd031-ebb5_118ed07435
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [public diagram tooling](../../scripts/docs/lib/public-diagrams.mjs).
- **Assessment:** The tooling checks raw title and description attributes only
  for truthiness. Whitespace-only values therefore satisfy the nominal non-empty
  contract while producing no meaningful accessible name.
- **Minimum repair:** Require trimmed title and description values to be
  non-empty before annotating the SVG.
- **Validation:** Add whitespace-only title and description fixtures alongside
  valid metadata cases.

### 15. OTP attempt-cap safety lacks a database-backed concurrency test

- **Finding:** fnd_sig-feat-route-c6c95ee31e-9b089f_43160b2454
- **Scanner classification:** medium · test-gap
- **Evidence:** [OTP route](../../apps/anvil-api/src/routes/auth-otp.ts) and
  [mocked concurrency test](../../apps/anvil-api/src/__tests__/auth-otp.test.ts).
- **Assessment:** The JavaScript mock encodes the promised atomic behaviour and
  will pass even if the real query regresses to read-then-write. This finding is
  coverage debt, not proof that the current advisory-locked query races.
- **Minimum repair:** Add a PostgreSQL-backed integration test; leave route
  behaviour unchanged if the real query proves atomic.
- **Validation:** Run more than the maximum attempts concurrently against one
  active OTP and assert returned claims and stored attempts never exceed the
  cap.

### 16. Git extraction-to-evaluation conformance is excluded from normal tests

- **Finding:** fnd_sig-feat-test-suite-3ee4f472b1-6_9d9d7dc190
- **Scanner classification:** medium · test-gap
- **Evidence:**
  [conformance evaluation tests](../../crates/anvil-checks/tests/conformance_evaluate.rs).
- **Assessment:** Active tests construct evidence directly. The only case that
  drives GitExtractor and then evaluates the result is ignored because it
  depends on this repository's history.
- **Minimum repair:** Replace or supplement the dogfood case with a hermetic
  temporary Git repository and committed conventional-change fixture.
- **Validation:** Prove the non-ignored test extracts the selected commit and
  produces the expected Tier-0 verdict under the default cargo test command.

### 17. Uncontrolled benign cases can lower the measured false-positive rate

- **Finding:** fnd_sig-feat-test-suite-77ddfbae71-4_f4106d3c1b
- **Scanner classification:** medium · test-gap
- **Evidence:**
  [secret calibration runner](../../crates/anvil-checks/tests/secret_calibration.rs),
  [SDT module](../../plans/archive/modules/secret-detection-truth.aps.md), and
  [testing guide](../guides/testing.md).
- **Assessment:** A benign case without a declared non-vacuity control still
  enters benign_total. It can look clean simply because it never resembled a
  detectable secret, artificially improving the reported false-positive rate.
- **Minimum repair:** Require every benign case to declare a control and fail
  when uncontrolled_benign is non-empty.
- **Validation:** Add a control-less benign manifest case and prove the gate
  fails with its case ID.

### 18. Since mode ignores staged and unstaged selector options

- **Finding:** fnd_sig-feat-library-5982411632-4fab_42a7488950
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [Git status selection](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/anvil/runtime/src/watch/git-status.ts),
  [tests](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/anvil/runtime/src/watch/git-status.test.ts),
  and the [runtime maturity note](../../packages/anvil/runtime/README.md).
- **Assessment:** With since set, the implementation always returns the
  ref-based diff and never reads staged or unstaged flags. Callers cannot obtain
  the selection promised by the public options.
- **Minimum repair:** Define and implement combined-selector semantics or reject
  incompatible combinations and narrow the documentation.
- **Validation:** Cover every staged/unstaged combination with since set.

### 19. Ember proposal creation rejects after persistence when publish fails

- **Finding:** fnd_sig-feat-library-d492a0dec3-66c5_7f981bd76a
- **Scanner classification:** medium · contract-mismatch
- **Evidence:**
  [candidate service](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/edda-stack/src/ember/candidate-service.ts),
  [observation hook](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/edda-stack/src/ember/observation-hook.ts),
  and
  [tests](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/edda-stack/src/ember/candidate-service.test.ts).
- **Assessment:** The proposal is stored before event publication. If publish
  rejects, the caller sees failure after durable state changed and a natural
  retry can create another proposal. Ember remains a draft surface, which lowers
  urgency but does not remove the ambiguous outcome.
- **Minimum repair:** Choose a durable outbox or explicit created-with-pending-
  publication contract, and make retry idempotent.
- **Validation:** Force publish failure, inspect store and outbox state, retry,
  and prove no duplicate proposal is created.

### 20. The API README omits required GitHub OAuth credentials

- **Finding:** fnd_sig-feat-route-24c7d2a330-771f31_1ce2eab8d0
- **Scanner classification:** medium · docs-gap
- **Evidence:**
  [GitHub OAuth route](../../apps/anvil-api/src/routes/auth-github.ts) and the
  [API README](../../apps/anvil-api/README.md).
- **Assessment:** The environment table omits credentials required by hosted
  OAuth and CLI device flows. An operator following the owning README can deploy
  an authentication surface that fails at runtime.
- **Minimum repair:** Document the hosted OAuth and CLI credential names,
  purpose, provider, and which routes require them; do not include values.
- **Validation:** Cross-check the table against the credential helper modules
  and deployment configuration, then run the docs gate.

## P3 — advisory coverage, performance, and hygiene

### 21. Recognised and fallback authentication errors lack regression coverage

- **Finding:** fnd_sig-feat-route-9eb65fea2f-a2ad37_77eb4d489d
- **Scanner classification:** low · test-gap
- **Evidence:** [error page](../../apps/docs-shell/app/auth/error/page.tsx).
- **Assessment:** A typo in a recognised reason can silently fall back to the
  generic message. This is user-facing diagnosability debt, not an auth bypass.
- **Minimum repair:** Test one recognised reason and unknown or absent reasons;
  optionally extract a pure message resolver.
- **Validation:** Assert invalid_state maps to its specific message and unknown
  or absent values map to the generic message.

### 22. The pending-auth recovery page lacks regression coverage

- **Finding:** fnd_sig-feat-route-ea68cde701-4d56a1_c1562065ab
- **Scanner classification:** low · test-gap
- **Evidence:** [pending page](../../apps/docs-shell/app/auth/pending/page.tsx).
- **Assessment:** Recovery copy and the retry, support, and email-verification
  destinations can drift while typecheck and build remain green.
- **Minimum repair:** Add a focused rendered-component test.
- **Validation:** Assert the heading and all recovery destinations.

### 23. The docs landing page is forced dynamic without request data

- **Finding:** fnd_sig-feat-route-f44022f02c-73c870_78633640ed
- **Scanner classification:** low · risk
- **Evidence:** [docs landing page](../../apps/docs-shell/app/page.tsx).
- **Assessment:** force-dynamic disables full-route caching although the page
  contains no request-specific data. The cost is unnecessary server rendering,
  not incorrect content.
- **Minimum repair:** Remove force-dynamic or use bounded revalidation if
  calendar-year rollover without deployment is required.
- **Validation:** Confirm the production build classifies the route as static or
  deliberately revalidated.

### 24. The docs root route lacks behavioural coverage

- **Finding:** fnd_sig-feat-route-f44022f02c-6068aa_9490310487
- **Scanner classification:** low · test-gap
- **Evidence:** [docs landing page](../../apps/docs-shell/app/page.tsx).
- **Assessment:** Primary APS, kindling, and anvil destinations and accessible
  labels are not pinned by tests.
- **Minimum repair:** Add one rendered-component smoke test.
- **Validation:** Assert the three link names and destinations.

### 25. The API README omits the admin fleet endpoint

- **Finding:** fnd_sig-feat-route-25b003a83f-6b469c_711e4b12d8
- **Scanner classification:** low · docs-gap
- **Evidence:** [admin route](../../apps/anvil-api/src/routes/admin.ts) and the
  [API README](../../apps/anvil-api/README.md).
- **Assessment:** The implemented GET /api/v1/admin/fleet route is absent from
  the owning endpoint inventory. This is discovery drift, not a runtime defect.
- **Minimum repair:** Add the endpoint, authentication boundary, and response
  purpose to the existing table.
- **Validation:** Compare the README inventory with registered API routes and
  run the docs gate.

### 26. A JSON-mode integration test accepts non-JSON output

- **Finding:** fnd_sig-feat-test-suite-794320ab38-7_cc53786ee0
- **Scanner classification:** low · test-gap
- **Evidence:**
  [AST follow-up integration test](../../crates/anvil-cli/tests/ast_followup.rs).
- **Assessment:** The command requests JSON, but the test only searches stdout
  for RS-001. Human text or malformed JSON containing that token can pass.
- **Minimum repair:** Parse stdout and assert the structured finding code and
  path.
- **Validation:** Make invalid or human-formatted stdout fail the test.

### 27. New callers can bypass queued JS runtime lock waiters

- **Finding:** fnd_sig-feat-library-7c0d8094f1-a367_a4317f40f7
- **Scanner classification:** medium · confirmed-bug
- **Evidence:**
  [queue manager](https://github.com/eddacraft/anvil-001/blob/96bcaf39fd436ea5d68415dedc62c90976db1236/packages/anvil/runtime/src/concurrency/queue-manager.ts)
  and the [runtime maturity note](../../packages/anvil/runtime/README.md).
- **Assessment:** waitForLock attempts direct acquisition before honouring an
  existing queue. A new caller can overtake the queue head at release time. The
  defect is real, but this JS runtime path is winding down.
- **Minimum repair:** Coordinate direct acquisition with queue-head ownership,
  or permit direct acquisition only when the queue is empty.
- **Validation:** Queue A behind a held lock, release while starting B, and
  prove A always acquires first.

### 28. Secret-file tests leak temporary directories on assertion failure

- **Finding:** fnd_sig-feat-test-suite-f53827d8bc-e_ddafcd834c
- **Scanner classification:** low · risk
- **Evidence:**
  [secret file coverage tests](../../crates/anvil-checks/tests/secret_file_coverage.rs).
- **Assessment:** Manual cleanup occurs after assertions. A panic leaves unique
  directories containing files larger than 1 MiB in the system temporary
  directory.
- **Minimum repair:** Replace the helper and manual cleanup with tempfile
  TempDir or another RAII guard.
- **Validation:** Prove the directory disappears when the guard drops, including
  an unwind path if practical.

## Recommended sequencing

1. **Settings truth wave:** findings 2–5.
2. **API account and identity wave:** findings 6–9.
3. **Security-sensitive coverage wave:** findings 10–11 and 15.
4. **Small boundary fixes:** findings 1, 12, and 14.
5. **Docs tooling and operational docs:** findings 13, 20, and 25.
6. **Evaluation confidence:** findings 16–17 and 26.
7. **Maturity-scoped residuals:** findings 18–19 and 27.
8. **Docs-shell and test hygiene:** findings 21–24 and 28.

A promoted wave should revalidate its selected records against the then-current
main branch, name one owning APS module, and include focused validation
commands. This report deliberately does not create those work items.

## Disposition boundary

- **Included:** the 28 current open receipts created by the 2026-08-28 triage.
- **Excluded:** 21 duplicate or disproven records, 6 fixed records, 4 accepted
  boundaries, and the one concurrency claim still awaiting runtime proof.
- **No tracking mutation:** no APS item, CIB entry, GitHub issue, or release
  claim is created by this report.
- **No release gate:** P1 here is repair priority, not a statement that the
  current release candidate is blocked.
