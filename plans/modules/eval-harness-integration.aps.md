# Eval Harness Integration

| ID | Owner | Status |
|----|-------|--------|
| EVAL | @aneki | Ready |

## Purpose

Integrate an external eval harness through Anvil-owned contracts so trust regressions run in local workflows and CI without coupling core domain logic to framework internals.

## In Scope

- Eval harness adapter and execution contracts
- Suite definitions for trust/safety regression checks
- CI integration and baseline comparison reports
- Mapping eval outcomes to Anvil policy and evidence models

## Out of Scope

- Building a net-new eval framework
- Replacing OPA policy execution

## Interfaces

**Depends on:**
- `opa-enhancements`
- `opa-agent-orchestration`
- `drift-reporting`

**Exposes:**
- `EvalHarnessPort`
- `EvalRunSummary`
- `EvalRegressionReport`

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined

## Tasks

### EVAL-001: Define EvalHarnessPort
- **Intent:** Define a stable adapter contract for harness execution and result retrieval.
- **Expected Outcome:** Core domain depends on contract only.
- **Validation:** `pnpm nx test core --testNamePattern="eval harness port"`

### EVAL-002: Implement framework adapter
- **Intent:** Add a concrete adapter for harness suite execution.
- **Expected Outcome:** Harness suites run via adapter with normalized outputs.
- **Validation:** `pnpm nx test core --testNamePattern="eval harness adapter"`
- **Dependencies:** EVAL-001

### EVAL-003: Add CI regression command
- **Intent:** Make trust regressions part of standard CI checks.
- **Expected Outcome:** CI command emits pass/fail and delta summary.
- **Validation:** `pnpm nx test cli --testNamePattern="eval regression command"`
- **Dependencies:** EVAL-002

### EVAL-004: Persist canonical eval results
- **Intent:** Store evaluation outcomes in Anvil schema for trends and evidence use.
- **Expected Outcome:** Historical run data is queryable independent of framework format.
- **Validation:** `pnpm nx test storage --testNamePattern="eval result persistence"`
- **Dependencies:** EVAL-002

### EVAL-005: Link eval failures to policy guidance
- **Intent:** Connect eval regressions to remediation-oriented policy messages.
- **Expected Outcome:** Failures include policy context and recommended next actions.
- **Validation:** `pnpm nx test core --testNamePattern="eval policy guidance"`
- **Dependencies:** EVAL-003, EVAL-004

## Execution

Steps: [../execution/EVAL.steps.md](../execution/EVAL.steps.md)
