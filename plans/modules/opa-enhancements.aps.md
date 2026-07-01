# Policy Authoring and Runtime UX

| ID   | Owner | Priority | Status | Progress |
| ---- | ----- | -------- | ------ | -------- |
| OPAE | —     | high     | Draft  | 0/9      |

**Last reviewed:** 2026-07-02 (reset under
[`POLRESET`](./policy-value-enforcement-reset.aps.md)).

> **Reset note:** the old OPAE plan mixed a broad "delightful OPA" wishlist,
> retired TypeScript paths, natural-language generation, policy debugging,
> compliance reporting, remote bundles, and PR comments. That made OPAE look
> strategically important but blocked first policy value. This module is now the
> narrow product-contract home for **regorus-backed policy authoring and runtime
> UX**. Enterprise hierarchy, lifecycle, compliance reports, remote federation,
> AI governance signals, YAML/action taxonomy, and agent orchestration stay in
> their owning modules.

## Purpose

Make user-authored and bundled policies useful in the shipping Anvil product:
validate packs before load, evaluate through the ADR-040 regorus facade, explain
failures in remediation-first language, and provide the policy outcome contract
that save-time/pre-write enforcement can route to `warn`, `fence`, or
`interrupt`.

## In Scope

- User policy pack discovery and loading through `crates/anvil-policy`.
- Regorus-backed evaluation through `crates/anvil-policy-engine`; Go OPA remains
  reference/parity tooling only.
- Local policy library/install UX for starter packs.
- Remediation-first policy result and guidance contract.
- Deterministic save-time/pre-write policy input adapter over changed-code and
  graph context.
- Enforcement-routing contract that maps policy outcomes to Anvil's existing
  `warn`, `fence`, and `interrupt` vocabulary while preserving warnings-first
  defaults.
- User-facing docs and one starter example path.

## Out of Scope

- Natural-language policy generation.
- Interactive TUI policy debugger.
- Historical impact simulator.
- Remote bundle marketplace, federation, signing, or SSO.
- Broad PR auto-comments.
- Compliance reporting and legal framework coverage.
- Enterprise hierarchy/lifecycle/rollout management.
- Tool-call interception beyond existing save-time/pre-write write-validation
  surfaces.
- YAML/action taxonomy authoring; that remains ACTAX Phase 2 after the Rego
  path works end to end.

## Interfaces

**Depends on:**

- [POLRESET](./policy-value-enforcement-reset.aps.md) — reset sequence and
  enforcement design gate.
- [POLENG](../archive/modules/policy-engine.aps.md) / ADR-040 — regorus facade,
  `PolicyInput` v1, result post-processing, and `anvil policy eval` substrate.
- [POLVAL](./policy-pack-validation.aps.md) — pack metadata, manifests,
  validation, and test enforcement.
- [CPOL](./contextual-policy-assertions.aps.md) — deterministic context and
  guidance payloads.
- [IORISK](./io-risk-controls.aps.md) — shared risk vocabulary when starter packs
  need IO/prompt-risk dimensions.
- [EXCEPT](./git-native-exceptions.aps.md) — valid exception verification before
  fencing or interrupting.
- `crates/anvil-policy`, `crates/anvil-policy-engine`, `crates/anvil-cli`,
  `crates/anvil-intercept-rules`, and `crates/anvil-intercept-protocol`.

**Exposes:**

- User policy pack discovery contract.
- Policy library/install UX contract.
- Remediation-first policy guidance contract.
- Save-time/pre-write policy input adapter contract.
- Enforcement-routing contract for `warn`, `fence`, and `interrupt`.

## Acceptance Criteria

- [ ] User-authored Rego packs validate before evaluation.
- [ ] Valid packs evaluate through `anvil-policy-engine` / regorus, not a second
      production OPA runtime.
- [ ] Policy failures include rule id, source, rationale, changed-code context,
      and remediation or exception guidance.
- [ ] A starter policy pack can be installed locally and exercised from CLI and
      eval-regression fixtures.
- [ ] Save-time/pre-write policy results can route to `warn`, `fence`, or
      `interrupt` when explicitly configured.
- [ ] Default user posture stays warnings-first unless a policy or CI surface
      opts into stronger enforcement.
- [ ] Exceptions are checked for scope, expiry, attribution, and revocation before
      a policy result is suppressed.

## Work Items

### OPAE-001: Policy authoring reset ADR/spec

- **Status:** Proposed
- **Intent:** Pin the first-slice policy product contract and explicitly defer the
  old OPAE wishlist.
- **Expected Outcome:** ADR/spec records the Rego-first path, pack admission,
  save-time/pre-write boundary, exception requirement, and deferred surfaces.
- **Validation:** `pnpm adr:check` and `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Confidence:** high

### OPAE-002: User policy pack discovery contract

- **Status:** Proposed
- **Intent:** Define where local user/bundled policy packs live and how Anvil
  discovers them.
- **Expected Outcome:** Policy pack discovery is deterministic, workspace-scoped,
  and compatible with POLVAL manifests.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_pack_discovery`
- **Dependencies:** OPAE-001, POLVAL-001, POLVAL-002
- **Confidence:** high

### OPAE-003: Regorus-backed user policy load path

- **Status:** Proposed
- **Intent:** Load validated user policies through the ADR-040 policy-engine
  facade.
- **Expected Outcome:** User-authored Rego reaches regorus through the same facade
  as bundled packs; Go OPA remains reference/parity only.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- user_policy_eval`
- **Dependencies:** OPAE-002, POLVAL-004
- **Confidence:** high

### OPAE-004: Local policy library and install UX

- **Status:** Proposed
- **Intent:** Provide a local install/list/show path for starter packs without a
  remote marketplace.
- **Expected Outcome:** `anvil policy install` can install bundled starter packs
  into the local policy set with clear provenance.
- **Validation:** `cargo test -p eddacraft-anvil -- policy_install`
- **Dependencies:** OPAE-003, POLVAL-005
- **Confidence:** medium

### OPAE-005: Remediation-first policy guidance contract

- **Status:** Proposed
- **Intent:** Standardise policy failure output so policy breaches are actionable.
- **Expected Outcome:** Results include rule id, policy source, rationale,
  changed-code context, remediation, and exception guidance.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_guidance_contract`
- **Dependencies:** OPAE-003, CPOL-003
- **Confidence:** high

### OPAE-006: Save-time/pre-write policy input adapter

- **Status:** Proposed
- **Intent:** Build the deterministic policy input needed for changed-code policy
  evaluation at save-time/pre-write boundaries.
- **Expected Outcome:** Policy evaluation can consume changed paths, graph facts,
  config, and workflow context without whole-repo rescans on the hot path.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_prewrite_input`
- **Dependencies:** OPAE-003, CPOL-002, POLRESET-004
- **Confidence:** medium

### OPAE-007: Enforcement-routing contract

- **Status:** Proposed
- **Intent:** Map policy outcomes to Anvil's existing enforcement vocabulary.
- **Expected Outcome:** Explicit policy modes can route to `warn`, `fence`, or
  `interrupt`; default behaviour remains warnings-first.
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules -- policy_routing`
- **Dependencies:** OPAE-005, OPAE-006, EXCEPT-006, POLRESET-006
- **Confidence:** medium

### OPAE-008: Starter pack end-to-end proof

- **Status:** Proposed
- **Intent:** Prove one high-signal policy pack across install, validation,
  evaluation, guidance, and report-only regression.
- **Expected Outcome:** A starter pack demonstrates real policy value before broad
  compliance-pack expansion.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- starter_policy_pack`
  and `cargo test -p eddacraft-anvil -- eval_regression_command`
- **Dependencies:** OPAE-004, OPAE-007, POLRESET-007
- **Confidence:** medium

### OPAE-009: Policy authoring user docs

- **Status:** Proposed
- **Intent:** Explain the supported first-slice policy authoring path without
  promising deferred enterprise or AI-generation features.
- **Expected Outcome:** Public docs show how to author, validate, install, run,
  and exception a policy pack.
- **Validation:** `pnpm docs:check`
- **Dependencies:** OPAE-008
- **Confidence:** high
