# OPA Agent Orchestration

| ID | Owner | Status |
|----|-------|--------|
| OPAG | @aneki | Proposed |

**Last reviewed:** 2026-06-24 (policy-solution validation)

> **Status correction (2026-06-24):** demoted Ready → Proposed. OPAG depends
> on OPAE product contracts (`OPAE-001`, `OPAE-012`, exception workflow items)
> that are still Draft and TS-era. The correct substrate is
> ADR-040/POLENG (`anvil-policy-engine` over regorus) plus the frozen
> `anvil policy eval --json` v1 output; OPAG should orchestrate that substrate
> and EXCEPT-managed exceptions, not introduce a separate Go OPA runtime.
> Promote OPAG only after the OPAE contract slice is rewritten against
> Rust/regorus and the MCP/agent-facing surface is explicitly re-approved.

> NOTE(post-rust): Validation commands have been retargeted to the Rust
> workspace (`cargo test -p eddacraft-anvil-policy`,
> `cargo test -p eddacraft-anvil`, and related crates). Dependency modules
> `opa-architecture-integration`, `architecture-safety`, and `mcp-server` are
> archived; their capability is covered by `crates/anvil-policy` and
> `crates/anvil-kernel` (architecture invariants) respectively. MCP surface
> integration is parked.

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
- `opa-enhancements` — policy product contracts (must be Rust/regorus-retargeted
  before OPAG executes)
- `git-native-exceptions` — durable exception records and verification
- `crates/anvil-kernel` / `crates/anvil-architecture` — architecture and repo
  signal inputs
- MCP / agent-facing runtime surface — deferred until the surface is re-approved

**Exposes:**

- `PolicyOrchestrator` — checkpoint-based evaluation coordinator
- `PolicyGuidanceContract` — normalized remediation output schema
- `ExceptionWorkflow` — request/review/expiry lifecycle
- `PolicyAuditEvents` — append-only event stream for decisions

## Promotion Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [ ] OPAE contract slice is retargeted to Rust/regorus and promoted
- [ ] MCP/agent-facing surface dependency is re-approved or explicitly removed

## Work Items

### OPAG-001: Define orchestration contract
- **Intent:** Establish the canonical contract for agent-driven policy orchestration.
- **Expected Outcome:** A stable schema exists for checkpoints, inputs, outcomes, and event IDs.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- orchestration_contract`
- **Dependencies:** OPAE-001, OPAE-012
- **Confidence:** high

### OPAG-002: Implement checkpoint policy runner
- **Intent:** Execute policy evaluation at save, staged, and CI checkpoints with deterministic ordering.
- **Expected Outcome:** Checkpoint runs produce consistent results and metadata regardless of trigger surface.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- checkpoint_runner`
- **Dependencies:** OPAG-001
- **Confidence:** high

### OPAG-003: Add remediation-first guidance model
- **Intent:** Standardize policy failure outputs into actionable fix guidance.
- **Expected Outcome:** Violations include clear rationale, suggested next actions, and confidence metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_guidance`
- **Dependencies:** OPAG-001, OPAE-012
- **Confidence:** high

### OPAG-004: Introduce exception workflow lifecycle
- **Intent:** Add explicit request/review/approve/reject/expire states for policy exceptions.
- **Expected Outcome:** Exception state transitions are enforced and auditable.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- exception_workflow`
- **Dependencies:** OPAE-024, OPAE-025, OPAE-026
- **Confidence:** medium

### OPAG-005: Add audit event stream
- **Intent:** Persist immutable events for evaluations, recommendations, and exception decisions.
- **Expected Outcome:** Every policy decision has a traceable event chain.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- policy_audit_events`
- **Dependencies:** OPAG-002, OPAG-004
- **Confidence:** high

### OPAG-006: Integrate CLI/IDE/MCP/CI surface adapters
- **Intent:** Ensure each user surface renders consistent policy outcomes and guidance.
- **Expected Outcome:** Cross-surface parity exists for status, violation details, and remediation signals.
- **Validation:** `cargo test -p eddacraft-anvil -- surface_parity` (MCP surface deferred — module archived)
- **Dependencies:** OPAG-003, OPAG-005
- **Confidence:** medium

### OPAG-007: Add rollout controls and latency guardrails
- **Intent:** Gate rollout with feature flags and measurable performance budgets.
- **Expected Outcome:** Agent orchestration can be enabled progressively with monitored latency thresholds.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- orchestration_performance`
- **Dependencies:** OPAG-002
- **Confidence:** medium

## Execution

Steps: [../execution/OPAG.steps.md](../execution/OPAG.steps.md)
