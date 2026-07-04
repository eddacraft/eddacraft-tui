# Prompt Attack Regression Packs

| ID   | Owner  | Status      |
| ---- | ------ | ----------- |
| PATT | @aneki | In Progress |

**Last reviewed:** 2026-05-25 (APSCAN-010 canonical-heading migration)

## Purpose

Create reusable prompt-attack regression packs to validate resilience against
injection, exfiltration, and instruction-hijack scenarios before release.

## In Scope

- Attack scenario schema and fixture format
- Pack runner for local and CI use
- Severity scoring and fail-policy integration

## Work Items

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. UK spelling applied. -->

### PATT-001: Define attack scenario schema

- **Status:** Done
- **Intent:** Standardise prompt attack case representation.
- **Expected Outcome:** Scenarios encode payload, objective, and expected safe behaviour.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- attack_scenario_schema`
- **Validated:** `crates/anvil-kernel-types/src/attack_scenario.rs` — `AttackScenario`
  + `AttackCategory`/`SafeBehaviour` forward-compatible enums (serde-only, kebab-case,
  `#[serde(other)] Unknown`); severity reuses `RiskSeverity`. 8 filtered tests green,
  full crate + clippy `-D warnings` + fmt clean.

### PATT-002: Build attack pack runner

- **Status:** Ready
- **Intent:** Execute scenario packs deterministically across environments.
- **Expected Outcome:** Runner emits normalised outcomes and confidence metadata.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- attack_pack_runner`
- **Dependencies:** PATT-001

### PATT-003: Connect fail policy and CI gates

- **Status:** Ready
- **Intent:** Enforce configurable pass/fail thresholds by severity.
- **Expected Outcome:** CI can block or warn based on attack regression policy.
- **Validation:** `cargo test -p eddacraft-anvil -- attack_regression_gate`
- **Dependencies:** PATT-002

## Execution

Action plan: [../execution/PATT.actions.md](../execution/PATT.actions.md)
