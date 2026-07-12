# OPA Agent Orchestration

| ID | Owner | Status |
|----|-------|--------|
| OPAG | @aneki | Proposed |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`: re-scoped so a
pickup does not re-plan merged work; TS-era `OPAG.steps.md` deleted).

> **Reset posture (POLRESET-010 / ADR-098, 2026-07-04; gates restated
> 2026-07-11):** post-first-slice expansion — not a prerequisite for first
> policy value. The save-time/pre-write policy path **has shipped**
> (POLRESET-006 via #3165), so the only live gates are: (1) the MCP /
> agent-facing surface being explicitly re-approved, and (2) for anything
> touching agent tool calls, the ADR-098 AD-4 tool-call-interception ADR.
> Several work items below are now partially delivered under other modules —
> each carries a delta note. Coordinated by
> [`POLRESET`](../archive/modules/policy-value-enforcement-reset.aps.md).

> **Status correction (2026-06-24; reset 2026-07-02):** demoted Ready →
> Proposed. OPAG depends on the policy product contracts now reset under
> OPAE-001..009 and the cross-module POLRESET design gate. The correct substrate
> is ADR-040/POLENG (`anvil-policy-engine` over regorus) plus the frozen
> `anvil policy eval --json` v1 output; OPAG should orchestrate that substrate
> and EXCEPT-managed exceptions, not introduce a separate Go OPA runtime.
> 2026-07-11: the OPAE first slice and save-time/pre-write boundary **are
> accepted and shipped**; only the MCP/agent-facing surface re-approval
> remains from this note's conditions.

> NOTE(post-rust, corrected 2026-07-11): validation commands target the Rust
> workspace (`cargo test -p eddacraft-anvil-policy-engine`,
> `cargo test -p eddacraft-anvil`). Dependency modules
> `opa-architecture-integration`, `architecture-safety`, and `mcp-server` are
> archived; their capability is covered by `crates/anvil-policy-engine` and
> `crates/anvil-kernel` (architecture invariants). The **Rust MCP surface is
> live** (`crates/anvil-cli/src/mcp/`) and already carries the pre-write
> policy path (`policy_prewrite.rs`, #3165) — what is parked is agent-facing
> tool-call interception (AD-4), not MCP itself.

## Purpose

Define and deliver an orchestration layer that turns Anvil policy evaluation into an active, explainable agent workflow across local development and CI.

## In Scope

- Policy checkpoint orchestration (save/staged/CI)
- Evaluation context assembly and handoff to OPA
- Remediation-first violation outputs
- Exception workflow states and audit events
- Surface adapters for CLI, IDE, MCP, and CI

## Out of Scope

- Replacing Rego as the authoring language or replacing the
  `anvil-policy-engine` regorus facade selected by ADR-040.
- Adding a second production Go OPA runtime. Go OPA is reference/parity tooling
  only unless a future ADR reverses ADR-040.
- New language analyzers beyond existing Anvil support
- External identity/approval providers (SSO, HRIS)

## Interfaces

**Depends on:**

- POLENG / `crates/anvil-policy-engine` — regorus evaluation,
  `PolicyInput` v1, result post-processing, coverage/trace
- `opa-enhancements` — policy product contracts (OPAE-001..008 Done/Merged;
  only 009 docs / 010 pack config / 011 warm cache remain and none block OPAG)
- `policy-value-enforcement-reset` — satisfied: design gate (ADR-098) accepted
  and the first slice shipped (Done 10/10, 2026-07-05)
- `git-native-exceptions` — durable exception records and verification
- `crates/anvil-kernel` / `crates/anvil-architecture` — architecture and repo
  signal inputs
- MCP / agent-facing runtime surface — deferred until the surface is re-approved

**Exposes:**

- `PolicyOrchestrator` — checkpoint-based evaluation coordinator
- `PolicyGuidanceContract` — thin extension over the **shipped** OPAE-005
  guidance contract (`crates/anvil-policy-engine/src/guidance.rs`), not a
  parallel schema
- `ExceptionWorkflow` — request/review states layered on the **shipped**
  EXCEPT grant/revoke/verify lifecycle
- `PolicyAuditEvents` — extension of the **shipped** witness chain (ADR-037),
  not a new event stream

## Promotion Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] POLRESET design gate accepted and OPAE first slice promoted (ADR-098
      accepted 2026-07-04; OPAE-001..008 Done/Merged — satisfied 2026-07-11)
- [ ] MCP/agent-facing surface dependency is re-approved or explicitly removed
      (**the sole remaining gate**, plus the AD-4 interception ADR for any
      tool-call-facing item)

## Work Items

### OPAG-001: Define orchestration contract
- **Intent:** Establish the canonical contract for agent-driven policy orchestration.
- **Expected Outcome:** A stable schema exists for checkpoints, inputs, outcomes, and event IDs.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- orchestration_contract`
- **Dependencies:** POLRESET-001 (satisfied — ADR-098), OPAE-005 (Done —
  `guidance.rs`), OPAE-007 (Done — `policy_routing.rs` via #3165); on paper
  fully unblocked, gated only by the module-level surface re-approval
- **Confidence:** high

### OPAG-002: Implement checkpoint policy runner
- **Intent:** Orchestrate policy evaluation across save, staged, and CI
  checkpoints with deterministic ordering. **Delta note (2026-07-11):** the
  checkpoints themselves are shipped — save/pre-write via OPAE-006 +
  POLRESET-006 (#3165) and report-only CI via POLRESET-008 (#3170); residual
  scope is cross-checkpoint orchestration and parity, not the checkpoints.
- **Expected Outcome:** Checkpoint runs produce consistent results and metadata regardless of trigger surface.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- checkpoint_runner`
- **Dependencies:** OPAG-001
- **Confidence:** high

### OPAG-003: Extend the shipped remediation-first guidance model
- **Intent:** Consume and extend the **shipped** OPAE-005 guidance contract
  (`crates/anvil-policy-engine/src/guidance.rs`) with any
  orchestration-specific fields (confidence metadata, event linkage). This
  item previously planned that contract from scratch; it exists.
- **Expected Outcome:** Violations include clear rationale, suggested next actions, and confidence metadata — via one guidance contract, not two.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_guidance`
- **Dependencies:** OPAG-001, OPAE-005 (Done)
- **Confidence:** high

### OPAG-004: Introduce exception workflow lifecycle
- **Intent:** Add explicit request/review/approve/reject states for policy
  exceptions **on top of the shipped EXCEPT lifecycle** — grant/revoke/list/
  show/verify CLI, scope/expiry verification, L4 gate wiring, and the ADR-100
  committed-store trust model are all Merged; the residual is the
  request-and-review half.
- **Expected Outcome:** Exception state transitions are enforced and auditable.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- exception_workflow`
- **Dependencies:** EXCEPT-004 (Merged #3153), EXCEPT-005 (Merged #2413),
  EXCEPT-006 (Merged #3140) — all satisfied
- **Confidence:** medium

### OPAG-005: Extend the witness chain with orchestration audit events
- **Intent:** Persist immutable events for evaluations, recommendations, and
  exception decisions **as an extension of the shipped witness chain
  (ADR-037)**, not a new parallel event stream.
- **Expected Outcome:** Every policy decision has a traceable event chain.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- policy_audit_events`
- **Dependencies:** OPAG-002, OPAG-004
- **Confidence:** high

### OPAG-006: Integrate CLI/IDE/MCP/CI surface adapters
- **Intent:** Ensure each user surface renders consistent policy outcomes and guidance.
- **Expected Outcome:** Cross-surface parity exists for status, violation details, and remediation signals.
- **Validation:** `cargo test -p eddacraft-anvil -- surface_parity` (the Rust
  MCP surface is live at `crates/anvil-cli/src/mcp/` and already renders
  pre-write policy diagnostics; only agent-facing tool-call interception
  stays behind the AD-4 ADR)
- **Dependencies:** OPAG-003, OPAG-005
- **Confidence:** medium

### OPAG-007: Add rollout controls and latency guardrails
- **Intent:** Gate rollout with feature flags and measurable performance
  budgets. **Delta note (2026-07-11):** the safety substrate is shipped —
  the `ANVIL_POLICY_ENFORCEMENT` out-of-band kill switch and the fail-open
  `PrewriteBudget` (#3165, ADR-098 AD-5); residual scope is
  orchestration-layer flags and budgets composing with them.
- **Expected Outcome:** Agent orchestration can be enabled progressively with monitored latency thresholds.
- **Validation:** `cargo test -p eddacraft-anvil-policy-engine -- orchestration_performance`
- **Dependencies:** OPAG-002
- **Confidence:** medium

## Execution

The former `plans/execution/OPAG.steps.md` was deleted 2026-07-11: it was
authored against the retired TS workspace (`pnpm nx test core/cli/mcp-server`)
and contradicted this module's Rust validation targets. Regenerate an action
plan at execution time per `plans/aps-rules.md`.
