<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Starter and Compliance Packs

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| CPACKS | —     | high     | In Progress  | 7/10      |

**Last reviewed:** 2026-08-25 — CPACKS-011 filed In Progress (second starter
pack `anvil-control-examples` plus a durable per-member overlay). Prior 2026-08-23 — CPACKS-007 promoted Proposed -> Ready as an
**enabling change** under PR #4100, not by a direct instruction naming this
item. The operator promoted POLFIT-007, which coordinates CPACKS-006 and
CPACKS-007; its stated outcome is not deliverable while this item sits Draft,
so it was advanced with it.

**Both live items closed 2026-08-24.** CPACKS-007 Merged via #4113 (known-gaps
copy — the audit found it did not exist). CPACKS-006 Merged via #4107 (eval
wrappers). An earlier revision of this paragraph recorded CPACKS-006 as Blocked
and not selectable; #4107 landed the same day and superseded that.

**Open after that closure:** CPACKS-008 (expansion gate), **CPACKS-009** and
**CPACKS-010** (filed 2026-08-24 from planning council `council-9021df43`).
**CPACKS-011** Merged via #4137.
The first-wave residue is closed; the module is not. Any statement below dated
before 2026-08-24 that calls CPACKS-006/-007 "the live residue" is historical. Prior review 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: re-scoped. The
previous revision was last reviewed 2026-07-02, two days **before** the
starter pack it plans shipped, and still framed it as future work.)

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; re-scoped 2026-07-11):**
> the first wave **has shipped** — the embedded `anvil-baseline` starter pack
> landed via POLRESET-007 (PR #3167, proven end-to-end:
> install → admission → gate advisory → pre-write → report-only CI harness)
> with its install UX via OPAE-004. CPACKS-001..005 are recorded
> satisfied-by below. The live residue: **CPACKS-006** (wire anvil-baseline
> fixtures into the CI eval suite — `ci/eval/suites.json` still contains only
> the `arch_boundary` suite) and **CPACKS-007**'s known-gaps docs audit.
> Everything beyond (broad OWASP/SOC 2/ISO/GDPR/AI packs) remains
> post-first-slice expansion behind CPACKS-008. Coordinated by
> [`POLRESET`](../archive/modules/policy-value-enforcement-reset.aps.md).

> **Reset note:** the previous CPACKS draft tried to author six broad compliance
> packs (OWASP, SOC 2, ISO 27001, GDPR, NIST AI RMF, EU AI Act) against retired
> TypeScript fixture paths. That looked valuable but created false-compliance
> risk before pack validation, user-policy loading, and evidence semantics were
> proven. This module now starts with one or two high-signal **starter packs**
> that prove policy value through the ADR-040 regorus path. Broad compliance
> packs return only after POLVAL, OPAE, EVALCI, and COMPLY are ready.

## Purpose

Ship bundled policy packs that users can install, validate, evaluate, and use as
examples for their own policies. The first wave proves real policy value with a
small, deterministic pack before Anvil makes broader compliance claims.

## In Scope

- One high-signal starter pack for architecture/security policy value.
- Optional second starter pack only if it reuses the same infrastructure without
  broadening compliance claims.
- Pack manifests, metadata, fixtures, and tests that satisfy POLVAL.
- Regorus-backed execution through `crates/anvil-policy-engine`.
- Remediation-first guidance through OPAE/CPOL contracts.
- Eval-regression fixtures for report-only CI.
- Documentation that labels starter packs as engineering controls, not legal
  compliance certification.

## Out of Scope

- Six-pack compliance sweep before the starter path is proven.
- Legal interpretation of SOC 2, ISO 27001, GDPR, NIST AI RMF, or EU AI Act.
- Remote marketplace, federation, hierarchy, lifecycle, or paid-pack delivery.
- AI-specific packs that depend on AGOV trust/capability signals before those
  signals exist.
- A second production OPA runtime.

## Interfaces

**Depends on:**

<!-- 2026-07-11: every listed prerequisite for the first wave has shipped; none block CPACKS-006/007 today. -->
- [POLRESET](../archive/modules/policy-value-enforcement-reset.aps.md) — first-slice sequencing
  (Done 10/10, 2026-07-05).
- [POLVAL](../archive/modules/policy-pack-validation.aps.md) — metadata, manifest, validation,
  and test contract (Done).
- [OPAE](./opa-enhancements.aps.md) — policy install UX, regorus-backed user
  policy loading, guidance, and enforcement-routing contracts (OPAE-001..008
  Done; 009/010/011 remain but do not block CPACKS).
- [CPOL](../archive/modules/contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads (Done).
- [IORISK](../archive/modules/io-risk-controls.aps.md) — risk vocabulary when a starter pack
  covers IO/prompt risk (Done).
- [EVALCI](./eval-regression-ci-gate.aps.md) — report-only regression coverage
  (005/006 Merged via #3170 — the surface CPACKS-006 wires into).

**Exposes:**

- Bundled starter pack manifest and Rego policies under the pack location chosen
  by OPAE/POLVAL.
- Pack fixtures for local validation and eval-regression.
- Starter-pack documentation and known-gaps notes.

## Acceptance Criteria

- [x] The starter pack installs through the OPAE local install path
      (POLRESET-007 proof stage 1: real `anvil policy install` with verified
      sha256 provenance).
- [x] The pack validates with POLVAL with zero structural issues (proof stage
      2: `load_manifest`/`validate_pack`/`run_pack_tests`/`enforce_tests`
      green).
- [x] Every policy has at least one pass fixture and one fail fixture (pack's
      own Rego tests pass through the regorus facade).
- [x] The pack evaluates through regorus via `anvil-policy-engine` (proof
      stages 3–4: gate advisory + pre-write projection).
- [x] Failure output includes remediation-first guidance and exception
      guidance (proof stage 3: review + `anvil exception grant`
      sensitive-paths copy).
- [x] Eval-regression runs the pack in report-only mode — **met 2026-08-24**
      (CPACKS-006, PR #4107): `ci/eval/suites.json` carries
      `anvil_baseline_change_scope` and `anvil_baseline_sensitive_paths`, fed by
      wrappers under `policies/eval/` that re-express each pack rule's `warning`
      string set as v1 `findings`. The committed baseline records
      `warning_count: 1` for both, so the suites diff real output rather than an
      empty array.
- [x] Documentation avoids legal compliance over-claims (anvil-baseline is
      documented as an engineering-control pack across the policies tutorial
      and beta-testing guide; residual known-gaps audit = CPACKS-007).

## Work Items

### CPACKS-001: Starter pack scope decision

- **Status:** Done — satisfied by POLRESET-007 (Merged 2026-07-04 via
  PR #3167): the decision landed as the embedded `anvil-baseline` pack
  (`crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/`).
- **Intent:** Choose the first pack by signal quality and enforcement fit.
- **Expected Outcome:** The first pack is narrowed to checks Anvil can evaluate
  deterministically with low false-positive risk.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Confidence:** high

### CPACKS-002: Starter pack manifest and metadata

- **Status:** Done — satisfied by POLRESET-007 (PR #3167): `pack.yaml` ships
  with the embedded pack and admission runs through the POLVAL pipeline
  (`load_manifest`/`validate_pack`), proven in `starter_proof.rs`.
- **Intent:** Define the pack manifest, ownership, severity, tags, and known-gaps
  metadata using the POLVAL contract.
- **Expected Outcome:** The pack can be discovered and validated before
  evaluation.
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil -- policy_install_bundled_manifest_validates`
  (the old `-p eddacraft-anvil-policy -- starter_pack_manifest` citation
  predated PR-C; no starter-pack code lives in that crate)
- **Dependencies:** CPACKS-001, POLVAL-001, POLVAL-002
- **Confidence:** high

### CPACKS-003: Starter pack policies and fixtures

- **Status:** Done — satisfied by POLRESET-007 (PR #3167): the pack's own Rego
  tests pass through the regorus facade (`run_pack_tests`/`enforce_tests`,
  proof stage 2).
- **Intent:** Author the first deterministic starter policies and pass/fail
  fixtures.
- **Expected Outcome:** Policies evaluate through regorus and fixtures prove both
  allowed and violating examples.
- **Validation:** `cargo test -p eddacraft-anvil -- starter_policy_pack` (7
  tests, `starter_proof.rs` in the CLI crate) and `opa test policies/fixtures/`
  (Go-OPA **compat** check only — dev-time reference per ADR-098 AD-1, not a
  second runtime)
- **Dependencies:** CPACKS-002, OPAE-003
- **Confidence:** medium

### CPACKS-004: Starter pack install path

- **Status:** Done — satisfied by OPAE-004 (`anvil policy install <PACK-ID>`,
  `install --list`, `show`), proven end-to-end with verified sha256 provenance
  in POLRESET-007 proof stage 1 (PR #3167).
- **Intent:** Wire the starter pack into the local policy install/list/show UX.
- **Expected Outcome:** Users can install and inspect the starter pack without a
  remote marketplace.
- **Validation:** `cargo test -p eddacraft-anvil -- policy_install`
- **Dependencies:** CPACKS-003, OPAE-004
- **Confidence:** medium

### CPACKS-005: Guidance and exception copy

- **Status:** Done — satisfied by POLRESET-007 proof stage 3 (PR #3167): the
  live gate surfaces the pack's warning-class advisory with remediation-first
  guidance (review + `anvil exception grant` sensitive-paths copy).
- **Intent:** Ensure starter-pack failures explain why the policy fired and how to
  fix or exception the result.
- **Expected Outcome:** Findings include remediation-first guidance and valid
  exception instructions.
- **Validation:** `cargo test -p eddacraft-anvil -- starter_policy_pack` (the
  guidance assertions live in the CLI-crate proof; the old
  `-p eddacraft-anvil-policy` citation predated PR-C)
- **Dependencies:** CPACKS-003, OPAE-005, EXCEPT-005
- **Confidence:** high

### CPACKS-006: Eval-regression fixture integration

- **Status:** Merged 2026-08-24 via PR #4107. Ancestor of `origin/main`
  (`c916da49f`). Unblock path 2: eval wrappers under `policies/eval/` emit v1
  Finding objects so the harness diffs `findings` rather than the pack's
  `warning` string set.
- **Files:** `ci/eval/suites.json`, `ci/eval/baseline/history.jsonl`,
  `ci/eval/README.md`, `ci/eval/inputs/`,
  `policies/eval/anvil_baseline_change_scope.rego`,
  `policies/eval/anvil_baseline_sensitive_paths.rego`,
  `crates/anvil-policy/src/eval/port.rs`,
  `crates/anvil-cli/src/commands/policy/starter_proof.rs`
- **Intent:** Add anvil-baseline fixtures to the report-only eval-regression
  path: `ci/eval/suites.json` currently carries only the `arch_boundary`
  suite, so the starter pack has no CI regression coverage despite being
  proven exercisable through the frozen eval v1 harness (POLRESET-007 proof
  stage 5).
- **Expected Outcome:** Policy regressions in the starter pack are visible in
  CI without becoming a required hard-fail; the committed baseline gains
  one-record-per-suite entries for the pack's suites.
- **Validation:** `cargo test -p eddacraft-anvil -- starter_policy_pack` (the
  `..._change_scope_eval_wrapper_lockstep` and
  `..._sensitive_paths_eval_wrapper_lockstep` parity tests in
  `starter_proof.rs`) plus `opa test --verbose policies/fixtures/ policies/eval/`.
  Corrected 2026-08-24: the previous line cited
  `cargo test -p eddacraft-anvil -- eval_regression_command`, copied from
  EVALCI-006. That filter matches only the ten synthetic
  `eval_regression_command_*` unit tests, which never reference
  `anvil-baseline`, `ci/eval/suites.json`, or the wrappers — it could not have
  proven this item's outcome.
- **Dependencies:** CPACKS-003 (Done), EVALCI-005 (Merged via #3170)
- **Confidence:** medium

- **Decision (2026-08-24):** path 2, in the form that keeps the duplication
  out of the shipped pack. Wrappers live under `policies/eval/`, never in
  `starter_packs/`, so no future pack author inherits a requirement to write
  one; lockstep tests fail on drift within the fixtures they exercise
  (CPACKS-009 widens them). Paths 1
  (extend the eval record) and 3 (smoke-only, renamed) were not taken —
  path 1 would have reworked a crate ADR-098 AD-2 slates for deletion, and
  path 3 discarded coverage that turned out to be cheap to get.
  The pre-decision options list that stood here is removed as superseded.

### CPACKS-007: Starter pack docs — known-gaps residual

- **Status:** Merged 2026-08-24 via PR #4113. Ancestor of `origin/main`
  (`cd852f1e8`); the known-gaps section verified present on the merged tree.
  Audit done, copy written. The audit
  found the residual was **not** an audit: the non-compliance-posture copy did
  not exist. This item's own status claimed anvil-baseline was documented
  "across `docs/public/anvil/tutorials/policies.md` and
  `docs/public/anvil/beta-testing-guide.md`" — the beta guide contains **zero**
  occurrences of `anvil-baseline`, and neither the tutorial nor the policy
  command reference carried a single compliance, limitation, or known-gap
  statement. Added a "What this pack does not do" section to the CPACKS-owned
  tutorial: advisory-only, no OWASP/SOC 2/ISO 27001/GDPR mapping and no
  compliance claim, exactly what the two policies inspect, what they do not
  (file contents, anything outside the diff, data flow), the deliberate
  name-heuristic false positives, and the fixed thresholds. `verified_against`
  advanced 0.9.0-beta -> 0.9.7-beta after running all four tutorial commands
  against a locally built binary (9 policy tests pass). Promoted Ready
  2026-08-23 as an **enabling change** under
  PR #4100, not by a direct instruction naming this item. The operator promoted
  POLFIT-007, which coordinates CPACKS-006 and this item; POLFIT-007's stated
  outcome is not deliverable while this one sits Draft. Grounds for advancing
  it rather than re-scoping POLFIT-007: dependencies CPACKS-004/-005 are Done,
  scope was settled by the 2026-07-11 re-scope, and Intent, Expected Outcome,
  and Validation were already present — it had simply never been advanced. (Re-scoped 2026-07-11: the bulk is delivered —
  anvil-baseline is documented across `docs/public/anvil/tutorials/policies.md`
  and `docs/public/anvil/beta-testing-guide.md`; the residual is an audit that
  the known-gaps and non-compliance-posture copy is explicit and complete.)
- **Intent:** Audit and complete the known-gaps and non-compliance-posture
  documentation for the shipped starter pack.
- **Expected Outcome:** Users can adopt the starter pack without confusing it for
  legal compliance coverage; known gaps are stated, not implied.
- **Validation:** `pnpm docs:check`
- **Dependencies:** CPACKS-004 (Done), CPACKS-005 (Done)
- **Confidence:** high

### CPACKS-009: Eval-wrapper coverage cannot silently regress

- **Status:** Proposed — filed 2026-08-24 from the CPACKS-006 planning council
  (session `council-9021df43`), raised independently by two reviewers.
- **Intent:** Stop the CPACKS-006 blind spot from reopening for a pack member
  or a rule branch that nobody wrote a wrapper for.
- **Mutation evidence (2026-08-24, reproduced during the council):** deleting
  the `password` matcher from the shipped `sensitive_paths.rego` leaves
  `opa test` at **9/9 PASS**. The council reviewer reported the same for
  `credential`, `apikey`, `id_rsa`, `secret`, and `.github/actions/` — **6 of
  the rule's 10 matchers are silently deletable**. Only `token` and
  `.github/workflows/` are actually guarded. Both lockstep tests use a single
  fixture, so a pack-only deletion the fixture does not hit produces identical
  output on both sides and passes.
- **Expected Outcome:** Three gaps close. (a) The pack tests and both lockstep
  guards are table-driven over a `(path, expected_message)` table covering all
  ten `sensitive-paths` matchers, so deleting any one fails. (b) A structural
  test iterates `pack.yaml`'s `policies:` list against `ci/eval/suites.json` and
  fails when a member has no registered eval suite — today's two are covered by
  hand, so a third policy would ship with zero eval coverage silently.
  (c) `change_scope`'s hard band (>25 files) gains a lockstep fixture; only the
  soft band (12 files) is exercised today.
- **Why this matters despite the pack being advisory:** ADR-040 pins regorus as
  the sole engine, and a regorus upgrade that shifts `contains` / `lower` /
  set-comprehension semantics is exactly the silent-breakage class these suites
  exist to catch. A release reviewer seeing a green report-only step approves
  the bump without hand-checking the pack. Blast radius is capped — this rule is
  a review prompt, not a credential control; content detection is the separately
  tested `secret` check — but the nudge disappearing unnoticed is the failure.
- **Files:** `crates/anvil-cli/src/commands/policy/starter_proof.rs`,
  `ci/eval/suites.json`
- **Validation:** `cargo test -p eddacraft-anvil -- starter_policy_pack`
- **Dependencies:** CPACKS-006 (Merged via #4107)
- **Confidence:** high

### CPACKS-010: Re-run the falsification against the landed suites

- **Status:** In Progress 2026-08-25 — implemented, awaiting merge. Two tests
  land in `eval_regression.rs`, both driven off `ci/eval/suites.json` so they
  cover the **landed** suites rather than hand-copied fixtures:
  `every_landed_eval_suite_is_falsifiable` (every suite's real policy against
  its real committed input must produce findings, and losing them must block —
  so a suite added later is covered without anyone remembering to extend the
  test) and `landed_sensitive_paths_suite_detects_a_neutered_matcher` (mutates
  the real wrapper Rego in memory, then drives the result through the real
  verdict *and* persistence paths, asserting the silence cannot become the
  accepted baseline).
- **RED-proven, twice.** Neutering `output_changed()` fails both tests.
  Pointing a suite's input at a path the policy ignores fails the first with
  "suite `anvil_baseline_sensitive_paths` produced no findings against its
  committed input" — which is CPACKS-006's original defect, now caught by name.
  Promoted Ready 2026-08-25 by operator instruction; dependencies CPACKS-006
  (#4107) and EVALCI-010 (#4128) both satisfied.
- **Intent:** Close the loop on the specific proof that motivated CPACKS-006,
  rather than inferring it from the parity and diff-engine tests.
- **Expected Outcome:** A committed test proves the landed suites detect a
  policy going silent, closing the loop on the falsification that opened
  CPACKS-006. The procedure is known and was run by hand while implementing
  EVALCI-010 — neuter a matcher in an eval wrapper, confirm the run reports
  `Δ … resolved:1` and exits non-zero under `--fail-on-regression`, confirm
  `history.jsonl` is unchanged, then confirm the re-run still detects. It is
  that hand-run which caught the baseline-poisoning bug in #4128 that 19 unit
  tests missed, so committing it is the point: the unit tests passed either
  way.
- **Shape (resolved 2026-08-24):** EVALCI-010 delivered `output_changed()`,
  the distinct loss-of-finding verdict. The alternative — an expected-output
  invariant — is no longer needed.
- **Files:** `crates/anvil-cli/src/commands/policy/`, `ci/eval/`
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression`
- **Dependencies:** CPACKS-006 (Merged via #4107), EVALCI-010 (Merged via
  #4128) — both satisfied
- **Confidence:** medium

### CPACKS-008: Compliance-pack expansion gate

- **Status:** Proposed
- **Intent:** Define the conditions for reintroducing OWASP/SOC2/ISO/GDPR/AI
  framework packs.
- **Expected Outcome:** Broad compliance packs remain blocked until validation,
  evaluation, evidence, and reporting contracts are proven.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** CPACKS-006, COMPLY-001 (Draft — COMPLY's
  evidence-semantics design gate is the real blocker), POLRESET-010
  (satisfied — Merged 2026-07-04 via PR #3134)
- **Confidence:** high

### CPACKS-011: Second starter pack with per-member overlay

- **Status:** Merged 2026-08-25 via PR #4137. Ancestor of `origin/main`
  (`2758a53ab`). Operator-approved design: one bundled pack, four members,
  durable overlay, no TUI this slice.
- **Intent:** Ship a second bundled starter pack that evaluates as real pack
  members, so custom policy authoring is a working loop rather than a docs
  recipe, without making GDPR or AI Act certification claims.
- **Expected Outcome:** `anvil policy install anvil-control-examples` writes
  `.anvil/policies/anvil-control-examples/` with four members.
  `crypto-human-signoff` emits the `violation` family and vetoes MCP pre-write
  under the default interrupt posture until `anvil exception grant --policy
  crypto-human-signoff` (existing store). The other three emit `warning` and
  never veto. A sibling overlay (`.anvil/policies/<pack>.overlay.yaml`) survives
  reinstall; disabled members are not evaluated by gate or MCP. CLI can list and
  set the overlay. Public copy labels the pack as engineering templates, not
  legal certification.
- **Scope:** Pack files, `BUNDLED_PACKS` registration, overlay load/filter on
  gate and MCP, exception suppression on MCP pre-write, members CLI, pack tests,
  public docs.
- **Non-scope:** TUI install picker; SETINS/SETGOV edit screen; YAML authoring
  (ADR-130); a new exception store; OWASP/SOC 2/ISO/GDPR/AI framework packs
  (CPACKS-008); bumping stored `N/M` counts.
- **Files:** `crates/anvil-cli/src/commands/policy/starter_packs/anvil-control-examples/`,
  `crates/anvil-cli/src/commands/policy/install.rs`,
  `crates/anvil-policy-engine/src/pack/`,
  `crates/anvil-cli/src/mcp/policy_prewrite.rs`,
  `crates/anvil-cli/src/commands/gate.rs`,
  `docs/public/anvil/concepts/policy-model.md`,
  `docs/public/anvil/reference/policy.md`,
  `docs/public/anvil/tutorials/policies.md`
- **Validation:** `cargo test -p eddacraft-anvil --bin anvil --no-fail-fast -- control_examples`
  and `cargo test -p eddacraft-anvil --bin anvil --no-fail-fast -- policy_install`
  and `pnpm docs:check` and `pnpm aps:active-lint`
- **Dependencies:** CPACKS-001..005 (Done); CPACKS-008 does not block — this is
  the optional second starter pack in module scope, not a framework pack.
- **Confidence:** high
