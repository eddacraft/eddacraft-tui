# OPA Agent Orchestration

| ID | Owner | Status |
|----|-------|--------|
| OPAG | @aneki | Ready |

## Purpose

Define and deliver an orchestration layer that turns Anvil policy evaluation into an active, explainable agent workflow across local development and CI.

## In Scope

- Policy checkpoint orchestration (save/staged/CI)
- Evaluation context assembly and handoff to OPA
- Remediation-first violation outputs
- Exception workflow states and audit events
- Surface adapters for CLI, IDE, MCP, and CI

## Out of Scope

- Replacing OPA/Rego as evaluation engine
- New language analyzers beyond existing Anvil support
- External identity/approval providers (SSO, HRIS)

## Interfaces

**Depends on:**

- `opa-architecture-integration` — OPA execution and policy input conventions
- `opa-enhancements` — policy packs, violation semantics
- `architecture-safety` — dependency/architecture signal inputs
- `mcp-server` — agent-facing runtime surface

**Exposes:**

- `PolicyOrchestrator` — checkpoint-based evaluation coordinator
- `PolicyGuidanceContract` — normalized remediation output schema
- `ExceptionWorkflow` — request/review/expiry lifecycle
- `PolicyAuditEvents` — append-only event stream for decisions

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Tasks

### OPAG-001: Define orchestration contract
- **Intent:** Establish the canonical contract for agent-driven policy orchestration.
- **Expected Outcome:** A stable schema exists for checkpoints, inputs, outcomes, and event IDs.
- **Validation:** `pnpm nx test core --testNamePattern="orchestration contract"`
- **Dependencies:** OPAE-001, OPAE-012
- **Confidence:** high

### OPAG-002: Implement checkpoint policy runner
- **Intent:** Execute policy evaluation at save, staged, and CI checkpoints with deterministic ordering.
- **Expected Outcome:** Checkpoint runs produce consistent results and metadata regardless of trigger surface.
- **Validation:** `pnpm nx test core --testNamePattern="checkpoint runner"`
- **Dependencies:** OPAG-001
- **Confidence:** high

### OPAG-003: Add remediation-first guidance model
- **Intent:** Standardize policy failure outputs into actionable fix guidance.
- **Expected Outcome:** Violations include clear rationale, suggested next actions, and confidence metadata.
- **Validation:** `pnpm nx test core --testNamePattern="policy guidance"`
- **Dependencies:** OPAG-001, OPAE-012
- **Confidence:** high

### OPAG-004: Introduce exception workflow lifecycle
- **Intent:** Add explicit request/review/approve/reject/expire states for policy exceptions.
- **Expected Outcome:** Exception state transitions are enforced and auditable.
- **Validation:** `pnpm nx test core --testNamePattern="exception workflow"`
- **Dependencies:** OPAE-024, OPAE-025, OPAE-026
- **Confidence:** medium

### OPAG-005: Add audit event stream
- **Intent:** Persist immutable events for evaluations, recommendations, and exception decisions.
- **Expected Outcome:** Every policy decision has a traceable event chain.
- **Validation:** `pnpm nx test core --testNamePattern="policy audit events"`
- **Dependencies:** OPAG-002, OPAG-004
- **Confidence:** high

### OPAG-006: Integrate CLI/IDE/MCP/CI surface adapters
- **Intent:** Ensure each user surface renders consistent policy outcomes and guidance.
- **Expected Outcome:** Cross-surface parity exists for status, violation details, and remediation signals.
- **Validation:** `pnpm nx test cli && pnpm nx test mcp-server`
- **Dependencies:** OPAG-003, OPAG-005
- **Confidence:** medium

### OPAG-007: Add rollout controls and latency guardrails
- **Intent:** Gate rollout with feature flags and measurable performance budgets.
- **Expected Outcome:** Agent orchestration can be enabled progressively with monitored latency thresholds.
- **Validation:** `pnpm nx test core --testNamePattern="orchestration performance"`
- **Dependencies:** OPAG-002
- **Confidence:** medium

## Execution

Steps: [../execution/OPAG.steps.md](../execution/OPAG.steps.md)
