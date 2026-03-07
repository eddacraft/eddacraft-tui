# Prompt Attack Regression Packs

| ID | Owner | Status |
|----|-------|--------|
| PATT | @aneki | Ready |

## Purpose

Create reusable prompt-attack regression packs to validate resilience against injection, exfiltration, and instruction-hijack scenarios before release.

## In Scope

- Attack scenario schema and fixture format
- Pack runner for local and CI use
- Severity scoring and fail-policy integration

## Tasks

### PATT-001: Define attack scenario schema
- **Intent:** Standardize prompt attack case representation.
- **Expected Outcome:** Scenarios encode payload, objective, and expected safe behavior.
- **Validation:** `pnpm nx test contracts --testNamePattern="attack scenario schema"`

### PATT-002: Build attack pack runner
- **Intent:** Execute scenario packs deterministically across environments.
- **Expected Outcome:** Runner emits normalized outcomes and confidence metadata.
- **Validation:** `pnpm nx test core --testNamePattern="attack pack runner"`
- **Dependencies:** PATT-001

### PATT-003: Connect fail policy and CI gates
- **Intent:** Enforce configurable pass/fail thresholds by severity.
- **Expected Outcome:** CI can block or warn based on attack regression policy.
- **Validation:** `pnpm nx test cli --testNamePattern="attack regression gate"`
- **Dependencies:** PATT-002

## Execution

Steps: [../execution/PATT.steps.md](../execution/PATT.steps.md)
