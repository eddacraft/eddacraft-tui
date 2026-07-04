# Policy Value and Enforcement Reset

| ID       | Type      | Owner | Priority | Status   | Progress |
| -------- | --------- | ----- | -------- | -------- | -------- |
| POLRESET | Conductor | —     | high     | In Progress | 0/10     |

**Last reviewed:** 2026-07-04 (POLRESET-001 design gate landed as ADR-098 via
planning council plan-18c47503).

## Purpose

Reset Anvil's policy roadmap around one combined outcome: teams can author
deterministic policies, validate them before use, see useful policy regressions
in CI, and opt into save-time or pre-write enforcement that can warn, fence, or
interrupt unsafe changes without adding a second policy runtime.

This is a **conductor** module. It coordinates existing policy modules rather
than replacing their execution authority. Work remains in the owning modules;
POLRESET defines the release-worthy sequence, design gates, dependency order, and
scope boundaries.

## Product Outcome

The first policy-value slice is complete when:

- user-authored Rego policy packs load through the ADR-040 regorus facade;
- policy packs fail fast on validation errors before evaluation;
- policy failures explain what changed, why it breaches policy, and how to fix or
  request an exception;
- scoped, expiring exceptions are honoured and auditable;
- at least one high-signal starter pack proves end-to-end policy value;
- report-only CI protects policy regressions with committed fixtures and a
  baseline;
- an opt-in enforcement mode can route a policy breach to `warn`, `fence`, or
  `interrupt` through the existing intercept vocabulary.

## Scope

### In Scope

- Retargeting stale policy modules to the Rust/regorus policy path
  (`crates/anvil-policy`, `crates/anvil-policy-engine`, `crates/anvil-cli`, and
  `crates/anvil-intercept-*`).
- A first-wave policy pack validation contract.
- Regorus-backed user policy discovery, loading, validation, and install UX.
- Deterministic save-time / pre-write policy context over changed code and graph
  facts already available to Anvil.
- Policy outcome routing to `warn`, `fence`, and `interrupt`, reconciled with
  ADR-002's warnings-first default.
- Exception verification and audit integration as a prerequisite for blocking
  policy modes.
- CI regression coverage for the shipped policy path.

### Out of Scope

- A second production Go OPA runtime. Go OPA remains reference/parity tooling.
- Tool-call interception beyond existing save-time / pre-write write-validation
  surfaces; that needs its own ADR before ACTAX Phase D or OPAG agent surfaces
  can execute.
- Natural-language policy generation, TUI debugger, impact simulator, broad PR
  comments, remote bundle marketplace, and enterprise hierarchy/lifecycle work in
  the first slice.
- Legal compliance claims before pack validation, starter packs, evidence
  semantics, and reporting are proven.

## Design Gates

1. **Policy admission and enforcement ADR:** reconcile ADR 002, ADR 015,
   ADR 037, and ADR 040 for user-authored policy admission, validation-before-load,
   exceptions, and `warn` / `fence` / `interrupt` routing.
2. **Pre-write boundary decision:** confirm that the first slice uses existing
   save-time / pre-write write-validation surfaces, not a new tool-call
   interception layer.
3. **CI blocking posture ADR:** required before EVALCI-008 can make policy
   regression a required hard-fail.

## Coordinated Modules

| Module | Role in Reset | Reset Posture |
| ------ | ------------- | ------------- |
| [policy-pack-validation](./policy-pack-validation.aps.md) | validates packs before evaluation | Retarget to Rust/regorus and promote first |
| [opa-enhancements](./opa-enhancements.aps.md) | policy authoring/runtime UX contracts | Reset from broad OPAE wishlist to narrow regorus-backed UX |
| [contextual-policy-assertions](./contextual-policy-assertions.aps.md) | deterministic context and guidance | Keep Ready; coordinate schema overlap with OPAE/ACTAX |
| [io-risk-controls](./io-risk-controls.aps.md) | shared risk taxonomy for policy outcomes | Keep Ready; use when starter pack needs IO/prompt risk |
| [git-native-exceptions](./git-native-exceptions.aps.md) | scoped, expiring, auditable exceptions | Required before blocking/fencing policy modes |
| [compliance-policy-packs](./compliance-policy-packs.aps.md) | starter and later compliance packs | Reset first wave to one or two high-signal starter packs |
| [eval-regression-ci-gate](./eval-regression-ci-gate.aps.md) | report-only, then blocking policy regression CI | Promote EVALCI-005/006 after starter policy path exists |
| [adversarial-testing-catalog](./adversarial-testing-catalog.aps.md) | adversarial policy/eval depth | Execute after report-only eval path |
| [prompt-attack-regression-packs](./prompt-attack-regression-packs.aps.md) | prompt attack regression depth | Execute after ATC/eval substrate |
| [policy-action-taxonomy](./policy-action-taxonomy.aps.md) | YAML/action taxonomy and risk-score authoring | Phase 2; do not block first Rego-backed value slice |
| [opa-agent-orchestration](./opa-agent-orchestration.aps.md) | agent-facing orchestration | Keep Proposed until save-time path and agent surface are re-approved |
| [org-policy-hierarchy](./org-policy-hierarchy.aps.md) | enterprise hierarchy | Later enterprise expansion |
| [policy-lifecycle](./policy-lifecycle.aps.md) | policy rollout lifecycle | Later enterprise expansion |
| [compliance-reporting](./compliance-reporting.aps.md) | compliance evidence/reporting | Later, after packs and evidence semantics |
| [agent-governance-patterns](./agent-governance-patterns.aps.md) | agent trust/capability signals | Later signal producers, not first-slice prerequisites |

## Work Items

These work items coordinate the reset. Implementation still lands in the owning
modules named by each `Coordinates with` field.

### POLRESET-001: Policy value and enforcement design gate

- **Status:** Merged 2026-07-04 via PR #3121
- **Intent:** Produce the accepted ADR/spec that resets policy module boundaries,
  first-slice scope, and enforcement semantics.
- **Expected Outcome:** A decision record pins the Rego-first path, validation
  before load, exception requirements, pre-write boundary, and `warn` / `fence` /
  `interrupt` mapping.
- **Decision Record:** [ADR-098 — Policy Enforcement Reset Gate](../decisions/098-policy-enforcement-reset-gate.md),
  produced via planning council plan-18c47503 (operator-ratified all gate
  questions). Ratifies ADR-015 as bookkeeping.
- **Validation:** `pnpm adr:check` and `pnpm aps:active-lint`
- **Dependencies:** ADR 002, ADR 015, ADR 037, ADR 040
- **Coordinates with:** OPAE, POLVAL, CPOL, IORISK, EXCEPT, ACTAX, OPAG
- **Confidence:** high

### POLRESET-002: Policy pack validation foundation

- **Status:** Merged 2026-07-04 via PR #3138
- **Intent:** Retarget POLVAL to the Rust/regorus path and promote its first wave
  when the design gate lands.
- **Expected Outcome:** Pack metadata, manifests, tests, CLI validation, and gate
  preflight are executable through `crates/anvil-policy` and the policy-engine
  facade.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_pack_validator`
  and `cargo test -p eddacraft-anvil -- policy_validate`
- **Dependencies:** POLRESET-001
- **Coordinates with:** POLVAL-001..005
- **Confidence:** high

### POLRESET-003: OPAE product-contract reset

- **Status:** Merged 2026-07-04 via PR #3136
- **Intent:** Replace OPAE's stale broad wishlist with first-wave policy
  authoring, loading, install, guidance, and enforcement-routing contracts.
- **Expected Outcome:** OPAE exposes only the contracts needed for policy value and
  save-time enforcement; non-essential UX is explicitly deferred.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Coordinates with:** OPAE-001..009
- **Confidence:** high

### POLRESET-004: Deterministic policy context and risk vocabulary

- **Status:** Merged 2026-07-04 via PR #3139
- **Intent:** Ensure policy evaluation receives deterministic changed-code,
  workflow, graph, and risk context before enforcement routing is attempted.
- **Expected Outcome:** CPOL and IORISK provide reusable input contracts for
  policy packs and save-time/pre-write evaluation.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- assertion_context`
  and `cargo test -p eddacraft-anvil-policy-engine -- io_risk_guidance`
- **Dependencies:** POLRESET-001
- **Coordinates with:** CPOL-001..003, IORISK-001..003, ACTAX schema overlap
- **Confidence:** medium

### POLRESET-005: Exception verification before blocking

- **Status:** Done — satisfied by the EXCEPT chain: verification
  (`verify_exception_at`, EXCEPT-005 via PR #2413), write-path hardening
  (EXCEPT-007, PR #2366, shipped v0.8.0-beta), and gate application +
  use-recording at the L4 evaluation seam (EXCEPT-006 via PR #3140,
  2026-07-04 council re-scope: L4 is the only rule-evaluation seam; L3
  inherits when scanner integration lands). Evidence re-verified
  2026-07-04: `cargo test -p eddacraft-anvil-policy -- exception` 65
  green; `cargo test -p eddacraft-anvil-l4 -- exceptions` 10 green.
- **Intent:** Make exceptions scoped, expiring, attributed, and enforced before
  any policy mode can fence or interrupt.
- **Expected Outcome:** Valid exceptions suppress only matching findings; use is
  recorded and invalid/expired/revoked exceptions degrade or fail safely.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_verify` and
  `cargo test -p eddacraft-anvil-policy -- exception`
- **Dependencies:** EXCEPT-005, EXCEPT-006, EXCEPT-007
- **Coordinates with:** EXCEPT-004..009, OPAE-007
- **Confidence:** high

### POLRESET-006: Save-time policy enforcement routing

- **Status:** Done — the routing contract (OPAE-007,
  `crates/anvil-intercept-rules/src/policy_routing.rs`) plus its MCP pre-write
  consumer (`crates/anvil-cli/src/mcp/policy_prewrite.rs`) connect regorus-backed
  policy results to the unified kernel-types `ControlDecision` vocabulary without
  touching the daemon boundary. Pre-write evaluation runs AFTER the intercept-rules
  scan on `anvil_validate_write` (additive, never replacing it), gated by the
  `ANVIL_POLICY_ENFORCEMENT` out-of-band kill switch (AD-5; re-read per call,
  `off`/`0` bypasses `.anvil.yaml`), discovers packs via `discover_and_load`,
  evaluates each through the ADR-040 facade over the OPAE-006 `PrewriteInput`
  projection with its tight fail-open `PrewriteBudget`, and routes findings via
  OPAE-007. Fail-open per AD-5: a broken/unparseable pack, an uncompilable member,
  an eval error, a **budget timeout**, or a panic degrades that pack to a
  warning-class outcome (never a veto, never a crashed tool call). The routed
  decision merges strictest-wins with the scan decision; default posture stays
  warnings-first (ADR-002). Off-daemon boundary held: `cargo test -p
  eddacraft-anvil-intercept --test daemon_dep_boundary` (7) confirms no
  `regorus`/`anvil-policy*` reaches the resident daemon. Validated by
  `cargo test -p eddacraft-anvil-intercept-rules -- policy_routing` (6) and
  `cargo test -p eddacraft-anvil -- policy_prewrite_routing` (11: violation+interrupt
  vetoes, violation+warn does not, warn-family never vetoes, kill switch off yields
  no diagnostics, broken pack + uncompilable member warn not veto, budget-exhaustion
  degradation, strictest-wins merge, deadline-exhaustion truncation). The whole
  pre-write pass (discovery + compile + eval) runs under one wall-clock deadline
  and truncates fail-open; per-call discovery + compile is uncached today
  (warm-cache follow-up filed as OPAE-011).
- **Intent:** Connect regorus-backed policy results to Anvil's existing
  enforcement vocabulary without changing the daemon boundary contract.
- **Expected Outcome:** Policy outcomes can opt into `warn`, `fence`, or
  `interrupt` while default behaviour remains warnings-first per ADR 002.
- **Validation:** `cargo test -p eddacraft-anvil-intercept-rules -- policy_routing`
  and `cargo test -p eddacraft-anvil -- policy_prewrite_routing`
- **Dependencies:** POLRESET-002, POLRESET-004, POLRESET-005, OPAE-006
- **Coordinates with:** OPAE-007, ACTAX future risk-score fusion
- **Confidence:** medium

### POLRESET-007: Starter policy pack proof

- **Status:** Proposed
- **Intent:** Ship one high-signal starter pack that proves real policy value
  before broad compliance-pack expansion.
- **Expected Outcome:** A starter pack installs, validates, evaluates through
  regorus, emits remediation-first guidance, and can be exercised in report-only
  CI.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- starter_policy_pack`
  and `opa test policies/fixtures/`
- **Dependencies:** POLRESET-002, POLRESET-003, POLRESET-006
- **Coordinates with:** CPACKS, OPAE-004, OPAE-008
- **Confidence:** medium

### POLRESET-008: Report-only policy regression CI

- **Status:** Proposed
- **Intent:** Promote the already-hardened EVALCI path into visible report-only
  policy regression coverage.
- **Expected Outcome:** Every PR gets a non-blocking eval-regression report over a
  committed policy suite and baseline.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command` and
  workflow lint for `.github/workflows/rust-tests.yml`
- **Dependencies:** POLRESET-007, EVALCI-001..004
- **Coordinates with:** EVALCI-005, EVALCI-006
- **Confidence:** high

### POLRESET-009: Adversarial policy depth

- **Status:** Proposed
- **Intent:** Add adversarial and prompt-attack regression depth after the first
  report-only policy path is visible.
- **Expected Outcome:** ATC and PATT findings feed eval summaries without
  destabilising the first policy-value slice.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- adversarial_eval_integration`
  and `cargo test -p eddacraft-anvil -- attack_regression_gate`
- **Dependencies:** POLRESET-008
- **Coordinates with:** ATC-001..004, PATT-001..003
- **Confidence:** medium

### POLRESET-010: Enterprise policy backlog reset

- **Status:** Merged 2026-07-04 via PR #3134
- **Intent:** Reclassify hierarchy, lifecycle, reporting, federation, compliance,
  and agent-governance modules as post-first-slice expansion until their
  prerequisites are real.
- **Expected Outcome:** ORGHIER, POLLC, COMPLY, POLFED, CPACKS expansion, OPAG,
  AGOV, and ACTAX Phase 2 no longer appear to block first policy value.
- **Validation:** `pnpm aps:active-lint`
- **Dependencies:** POLRESET-001
- **Coordinates with:** ORGHIER, POLLC, COMPLY, POLFED, CPACKS, OPAG, AGOV, ACTAX
- **Confidence:** high
