<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Policy Starter and Compliance Packs

| ID     | Owner | Priority | Status | Progress |
| ------ | ----- | -------- | ------ | -------- |
| CPACKS | —     | high     | Draft  | 0/8      |

**Last reviewed:** 2026-07-02 (reset under
[`POLRESET`](./policy-value-enforcement-reset.aps.md)).

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

- [POLRESET](./policy-value-enforcement-reset.aps.md) — first-slice sequencing.
- [POLVAL](./policy-pack-validation.aps.md) — metadata, manifest, validation,
  and test contract.
- [OPAE](./opa-enhancements.aps.md) — policy install UX, regorus-backed user
  policy loading, guidance, and enforcement-routing contracts.
- [CPOL](./contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads.
- [IORISK](./io-risk-controls.aps.md) — risk vocabulary when a starter pack
  covers IO/prompt risk.
- [EVALCI](./eval-regression-ci-gate.aps.md) — report-only regression coverage.

**Exposes:**

- Bundled starter pack manifest and Rego policies under the pack location chosen
  by OPAE/POLVAL.
- Pack fixtures for local validation and eval-regression.
- Starter-pack documentation and known-gaps notes.

## Acceptance Criteria

- [ ] The starter pack installs through the OPAE local install path.
- [ ] The pack validates with POLVAL with zero structural issues.
- [ ] Every policy has at least one pass fixture and one fail fixture.
- [ ] The pack evaluates through regorus via `anvil-policy-engine`.
- [ ] Failure output includes remediation-first guidance and exception guidance.
- [ ] Eval-regression can run the pack in report-only mode.
- [ ] Documentation avoids legal compliance over-claims.

## Work Items

### CPACKS-001: Starter pack scope decision

- **Status:** Proposed
- **Intent:** Choose the first pack by signal quality and enforcement fit.
- **Expected Outcome:** The first pack is narrowed to checks Anvil can evaluate
  deterministically with low false-positive risk.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Confidence:** high

### CPACKS-002: Starter pack manifest and metadata

- **Status:** Proposed
- **Intent:** Define the pack manifest, ownership, severity, tags, and known-gaps
  metadata using the POLVAL contract.
- **Expected Outcome:** The pack can be discovered and validated before
  evaluation.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- starter_pack_manifest`
- **Dependencies:** CPACKS-001, POLVAL-001, POLVAL-002
- **Confidence:** high

### CPACKS-003: Starter pack policies and fixtures

- **Status:** Proposed
- **Intent:** Author the first deterministic starter policies and pass/fail
  fixtures.
- **Expected Outcome:** Policies evaluate through regorus and fixtures prove both
  allowed and violating examples.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- starter_policy_pack`
  and `opa test policies/fixtures/`
- **Dependencies:** CPACKS-002, OPAE-003
- **Confidence:** medium

### CPACKS-004: Starter pack install path

- **Status:** Proposed
- **Intent:** Wire the starter pack into the local policy install/list/show UX.
- **Expected Outcome:** Users can install and inspect the starter pack without a
  remote marketplace.
- **Validation:** `cargo test -p eddacraft-anvil -- policy_install`
- **Dependencies:** CPACKS-003, OPAE-004
- **Confidence:** medium

### CPACKS-005: Guidance and exception copy

- **Status:** Proposed
- **Intent:** Ensure starter-pack failures explain why the policy fired and how to
  fix or exception the result.
- **Expected Outcome:** Findings include remediation-first guidance and valid
  exception instructions.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- starter_pack_guidance`
- **Dependencies:** CPACKS-003, OPAE-005, EXCEPT-005
- **Confidence:** high

### CPACKS-006: Eval-regression fixture integration

- **Status:** Proposed
- **Intent:** Add starter-pack fixtures to the report-only eval-regression path.
- **Expected Outcome:** Policy regressions are visible in CI without becoming a
  required hard-fail.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command`
- **Dependencies:** CPACKS-003, EVALCI-005
- **Confidence:** medium

### CPACKS-007: Starter pack docs

- **Status:** Proposed
- **Intent:** Document starter-pack purpose, installation, examples, known gaps,
  and non-compliance posture.
- **Expected Outcome:** Users can adopt the starter pack without confusing it for
  legal compliance coverage.
- **Validation:** `pnpm docs:check`
- **Dependencies:** CPACKS-004, CPACKS-005
- **Confidence:** high

### CPACKS-008: Compliance-pack expansion gate

- **Status:** Proposed
- **Intent:** Define the conditions for reintroducing OWASP/SOC2/ISO/GDPR/AI
  framework packs.
- **Expected Outcome:** Broad compliance packs remain blocked until validation,
  evaluation, evidence, and reporting contracts are proven.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** CPACKS-006, COMPLY-001, POLRESET-010
- **Confidence:** high
