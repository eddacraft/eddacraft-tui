# IO Risk Controls

| ID | Owner | Status |
|----|-------|--------|
| IORISK | @aneki | Ready |

## Purpose

Introduce provider-agnostic input/output risk controls for prompt injection, sensitive data leakage, and unsafe response patterns.

## In Scope

- Input and output scanner contracts
- Risk taxonomy and severity model
- Policy integration for enforce/warn modes

## Tasks

### IORISK-001: Define IO risk taxonomy
- **Intent:** Standardize categories, severity, and confidence for IO risk findings.
- **Expected Outcome:** A consistent taxonomy is used across scanners and policy outputs.
- **Validation:** `pnpm nx test contracts --testNamePattern="io risk taxonomy"`

### IORISK-002: Implement scanner pipeline
- **Intent:** Add scanner execution pipeline for pre/post model checks.
- **Expected Outcome:** Input/output streams are evaluated through pluggable scanner chain.
- **Validation:** `pnpm nx test core --testNamePattern="io scanner pipeline"`
- **Dependencies:** IORISK-001

### IORISK-003: Integrate risk findings with policy outputs
- **Intent:** Map IO findings to policy outcomes and remediation actions.
- **Expected Outcome:** Findings appear in unified guidance and CI summaries.
- **Validation:** `pnpm nx test core --testNamePattern="io risk guidance"`
- **Dependencies:** IORISK-002

## Execution

Steps: [../execution/IORISK.steps.md](../execution/IORISK.steps.md)
