# Adversarial Testing Catalog

| ID | Owner | Status |
|----|-------|--------|
| ATC | @aneki | Ready |

**Last reviewed:** 2026-04-26

## Purpose

Build an Anvil-native catalog of adversarial test probes to continuously validate prompt safety, data handling, and model behavior regressions.

## In Scope

- Probe taxonomy and metadata model
- Reusable probe packs by risk category
- Probe execution hooks via eval harness integration
- Regression trend reporting for adversarial findings

## Tasks

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. Categorise UK English: standardise/categorise. EVAL-002 reference assumes eval-harness-integration module. -->

### ATC-001: Define adversarial probe taxonomy
- **Intent:** Standardise categories, payload classes, and expected outcomes.
- **Expected Outcome:** Probe catalog supports traceable and versioned test assets.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- adversarial_taxonomy`

### ATC-002: Implement probe pack registry
- **Intent:** Add loadable probe packs with versioned manifests.
- **Expected Outcome:** Probe sets can be selected by risk profile and context.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- probe_registry`
- **Dependencies:** ATC-001

### ATC-003: Integrate probe execution into eval harness
- **Intent:** Execute adversarial probes in CI and local eval runs.
- **Expected Outcome:** Probe outcomes appear in eval regression summaries.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- adversarial_eval_integration`
- **Dependencies:** ATC-002, EVAL-002

### ATC-004: Add adversarial trend reporting
- **Intent:** Surface probe pass/fail trends by category over time.
- **Expected Outcome:** Teams can spot recurring weak points and regressions.
- **Validation:** `cargo test -p eddacraft-anvil -- adversarial_trends`
- **Dependencies:** ATC-003

## Execution

Steps: [../execution/ATC.steps.md](../execution/ATC.steps.md)
