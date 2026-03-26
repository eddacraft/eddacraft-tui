# ADR-011: OPA Agent Orchestration for Anvil

## Status

Proposed

## Date

2026-03-07

## Context

Anvil already has strong policy evaluation primitives (OPA checks, architecture rules, policy packs), but policy authoring and remediation are still mostly command-driven and manual.

For teams shipping quickly with AI-assisted coding, policy value drops when:

- violations are discovered too late in the workflow,
- remediation guidance is inconsistent,
- exception handling becomes ad-hoc,
- and policy intent is not represented as an active assistant in day-to-day development.

We need an **OPA Agent** mode that can continuously translate policy intent into actionable guardrails, evaluate changes early, and provide explainable, auditable guidance.

## Decision

Introduce an **OPA Agent** capability in Anvil as an orchestration layer over existing OPA and architecture systems.

The OPA Agent will:

1. Generate and maintain policy evaluation context for a change set.
2. Run policy checks continuously at defined checkpoints (save, staged, CI).
3. Return remediation-first guidance (what failed, why, safest next action).
4. Route exception requests through explicit, auditable workflow states.
5. Emit machine-readable outputs for CLI, IDE, MCP, and CI surfaces.

The agent will orchestrate; it will not replace OPA as policy engine.

## Rationale

### 1. Preserve proven policy core

OPA remains the evaluator. The agent coordinates inputs, timing, explanation, and workflow decisions.

### 2. Move policy left without hard blocking by default

Agent checkpoints make policy visible earlier while preserving Anvil’s warning-first posture.

### 3. Improve developer trust and adoption

Remediation-focused responses and consistent exception handling reduce policy fatigue.

### 4. Strengthen auditability

Every evaluation, recommendation, and exception decision is traceable and attributable.

### 5. Reuse existing Anvil architecture

This builds on existing modules (OPA integration, architecture safety, command safety, MCP) instead of introducing a parallel policy subsystem.

## Consequences

### Positive

- Faster feedback loops on policy violations.
- Consistent guidance quality across local and CI workflows.
- Better policy lifecycle visibility for platform and compliance stakeholders.
- Lower operational risk from ad-hoc exceptions.

### Negative

- Additional orchestration complexity and state management.
- Potential performance overhead if checkpoints are too frequent.
- Risk of over-automation if exception policy is too permissive.

### Mitigations

- Start with explicit checkpoint policy and conservative defaults.
- Add latency budgets and incremental evaluation paths.
- Require structured approvals and expiry for exceptions.

## Scope Boundaries

In scope:

- Orchestration of policy runs and guidance output.
- Exception request/approval state model.
- Cross-surface output contracts (CLI/IDE/MCP/CI).
- Evaluation/audit event model.

Out of scope:

- Replacing Rego or OPA runtime.
- New language analyzers beyond existing platform capabilities.
- Enterprise IAM/SSO workflows (separate module).

## References

- [ADR-006: Hybrid Dependency-Cruiser + OPA Architecture](./006-hybrid-dc-opa.md)
- [OPA Enhancements Module](../modules/opa-enhancements.aps.md)
- [Anvil APS Index](../index.aps.md)
