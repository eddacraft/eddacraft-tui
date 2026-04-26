# Prompt Attack Regression Packs

| ID | Owner | Status |
|----|-------|--------|
| PATT | @aneki | Ready |

**Last reviewed:** 2026-04-26

## Purpose

Create reusable prompt-attack regression packs to validate resilience against injection, exfiltration, and instruction-hijack scenarios before release.

## In Scope

- Attack scenario schema and fixture format
- Pack runner for local and CI use
- Severity scoring and fail-policy integration

## Tasks

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. UK spelling applied. -->

### PATT-001: Define attack scenario schema
- **Intent:** Standardise prompt attack case representation.
- **Expected Outcome:** Scenarios encode payload, objective, and expected safe behaviour.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- attack_scenario_schema`

### PATT-002: Build attack pack runner
- **Intent:** Execute scenario packs deterministically across environments.
- **Expected Outcome:** Runner emits normalised outcomes and confidence metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- attack_pack_runner`
- **Dependencies:** PATT-001

### PATT-003: Connect fail policy and CI gates
- **Intent:** Enforce configurable pass/fail thresholds by severity.
- **Expected Outcome:** CI can block or warn based on attack regression policy.
- **Validation:** `cargo test -p eddacraft-anvil -- attack_regression_gate`
- **Dependencies:** PATT-002

## Execution

Steps: [../execution/PATT.steps.md](../execution/PATT.steps.md)
