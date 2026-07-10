<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Starter and Compliance Packs

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| CPACKS | —     | high     | In Progress  | 5/8      |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
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
> [`POLRESET`](./policy-value-enforcement-reset.aps.md).

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
- [POLRESET](./policy-value-enforcement-reset.aps.md) — first-slice sequencing
  (Done 10/10, 2026-07-05).
- [POLVAL](./policy-pack-validation.aps.md) — metadata, manifest, validation,
  and test contract (Done).
- [OPAE](./opa-enhancements.aps.md) — policy install UX, regorus-backed user
  policy loading, guidance, and enforcement-routing contracts (OPAE-001..008
  Done; 009/010/011 remain but do not block CPACKS).
- [CPOL](./contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads (Done).
- [IORISK](./io-risk-controls.aps.md) — risk vocabulary when a starter pack
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
- [ ] Eval-regression can run the pack in report-only mode — **partially met**:
      proven exercisable through the frozen `anvil policy eval --json` v1
      harness (proof stage 5), but the pack's fixtures are not yet in
      `ci/eval/suites.json` (CPACKS-006, the live item).
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

- **Status:** Ready (promoted 2026-07-11 — both dependencies shipped; this is
  the module's live item)
- **Intent:** Add anvil-baseline fixtures to the report-only eval-regression
  path: `ci/eval/suites.json` currently carries only the `arch_boundary`
  suite, so the starter pack has no CI regression coverage despite being
  proven exercisable through the frozen eval v1 harness (POLRESET-007 proof
  stage 5).
- **Expected Outcome:** Policy regressions in the starter pack are visible in
  CI without becoming a required hard-fail; the committed baseline gains
  one-record-per-suite entries for the pack's suites.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command`
  plus the new suite entries in `ci/eval/suites.json` evaluated by the
  report-only step
- **Dependencies:** CPACKS-003 (Done), EVALCI-005 (Merged via #3170)
- **Confidence:** medium

### CPACKS-007: Starter pack docs — known-gaps residual

- **Status:** Proposed (re-scoped 2026-07-11: the bulk is delivered —
  anvil-baseline is documented across `docs/public/anvil/tutorials/policies.md`
  and `docs/public/anvil/beta-testing-guide.md`; the residual is an audit that
  the known-gaps and non-compliance-posture copy is explicit and complete)
- **Intent:** Audit and complete the known-gaps and non-compliance-posture
  documentation for the shipped starter pack.
- **Expected Outcome:** Users can adopt the starter pack without confusing it for
  legal compliance coverage; known gaps are stated, not implied.
- **Validation:** `pnpm docs:check`
- **Dependencies:** CPACKS-004 (Done), CPACKS-005 (Done)
- **Confidence:** high

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
