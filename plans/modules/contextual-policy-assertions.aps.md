# Contextual Policy Assertions

| ID | Owner | Status |
|----|-------|--------|
| CPOL | @aneki | Ready |

## Purpose

Add contextual assertion rules that evaluate agent and workflow actions with richer runtime context while preserving Anvil policy-pack semantics.

## In Scope

- Assertion rule schema and execution model
- Context adapters for policy inputs
- Assertion violation explanations and remediation

## Tasks

### CPOL-001: Define assertion schema
- **Intent:** Create a schema for contextual policy assertions.
- **Expected Outcome:** Assertions support scoped conditions and outcomes.
- **Validation:** `pnpm nx test core --testNamePattern="assertion schema"`

### CPOL-002: Implement context adapters
- **Intent:** Populate assertions with workflow and runtime context.
- **Expected Outcome:** Assertions evaluate with deterministic context payloads.
- **Validation:** `pnpm nx test core --testNamePattern="assertion context"`
- **Dependencies:** CPOL-001

### CPOL-003: Add assertion guidance outputs
- **Intent:** Provide actionable failure explanations and fix guidance.
- **Expected Outcome:** Assertion failures map to remediation-first outputs.
- **Validation:** `pnpm nx test core --testNamePattern="assertion guidance"`
- **Dependencies:** CPOL-002

## Execution

Steps: [../execution/CPOL.steps.md](../execution/CPOL.steps.md)
