# ADR-012: Eval Harness Framework Adoption

## Status

Proposed

## Date

2026-03-07

## Context

Anvil needs repeatable trust and safety regression testing that runs in developer workflows and CI. Current policy checks are strong, but evaluation loops for prompt/task-level regressions are fragmented.

We need a practical way to:

- define trust/safety eval suites,
- run them automatically in CI,
- compare results over time,
- and connect failures to policy and architecture controls.

## Decision

Adopt an external eval harness framework through an internal adapter boundary.

Adoption constraints:

1. Framework usage is isolated behind `EvalHarnessPort`.
2. Canonical persisted results use Anvil-owned schemas.
3. CI commands and developer workflows remain Anvil-first.
4. Adapter conformance tests are required to keep swappability.

## Rationale

### 1. Speed to value

A mature harness enables immediate trust regression coverage without inventing a new test ecosystem.

### 2. Low lock-in with proper boundaries

Port/adapter design keeps framework semantics out of core domain and policy models.

### 3. Better governance linkage

Eval outcomes can feed policy trends, exception reviews, and compliance evidence.

### 4. Developer adoption

A test-like model integrated into CI aligns with existing engineering habits.

## Consequences

### Positive

- Faster rollout of regression-based trust checks.
- Better confidence for AI-assisted changes.
- Stronger evidence trails for governance conversations.

### Negative

- Additional dependency surface and upgrade management.
- Potential mismatch between framework-native and Anvil-native result models.

### Mitigations

- Strict adapter contract with compatibility tests.
- Version pinning and controlled upgrade cadence.
- Normalization layer for evaluation output.

## References

- [Anvil APS Index](../index.aps.md)
- [OPA Agent Orchestration ADR](./011-opa-agent-orchestration.md)
