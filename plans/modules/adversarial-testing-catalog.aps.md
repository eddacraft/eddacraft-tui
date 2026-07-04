# Adversarial Testing Catalog

| ID  | Owner  | Status      |
| --- | ------ | ----------- |
| ATC | @aneki | In Progress |

**Last reviewed:** 2026-05-25 (APSCAN-004 canonical-heading migration)

## Purpose

Build an Anvil-native catalog of adversarial test probes to continuously
validate prompt safety, data handling, and model behaviour regressions.

## In Scope

- Probe taxonomy and metadata model
- Reusable probe packs by risk category
- Probe execution hooks via eval harness integration
- Regression trend reporting for adversarial findings

## Work Items

<!-- Audit 2026-04-26: Validation commands updated for Rust crates per ADR-026. Categorise UK English: standardise/categorise. EVAL-002 reference assumes eval-harness-integration module. -->

### ATC-001: Define adversarial probe taxonomy

- **Status:** Done
- **Intent:** Standardise categories, payload classes, and expected outcomes.
- **Expected Outcome:** Probe catalog supports traceable and versioned test assets.
- **Validation:** `cargo test -p eddacraft-anvil-kernel-types -- adversarial_taxonomy`
  (green — 8 tests). Added `crates/anvil-kernel-types/src/adversarial.rs`:
  `ProbeCategory`, `PayloadClass`, `ExpectedOutcome` (each with a `#[serde(other)]
  Unknown` fallback, kebab-case wire form) and the versioned `Probe` record.
  Serde-only, additive to the wire crate.

### ATC-002: Implement probe pack registry

- **Status:** Ready
- **Intent:** Add loadable probe packs with versioned manifests.
- **Expected Outcome:** Probe sets can be selected by risk profile and context.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- probe_registry`
- **Dependencies:** ATC-001

### ATC-003: Integrate probe execution into eval harness

- **Status:** Ready
- **Intent:** Execute adversarial probes in CI and local eval runs.
- **Expected Outcome:** Probe outcomes appear in eval regression summaries.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- adversarial_eval_integration`
- **Dependencies:** ATC-002, EVAL-002

### ATC-004: Add adversarial trend reporting

- **Status:** Ready
- **Intent:** Surface probe pass/fail trends by category over time.
- **Expected Outcome:** Teams can spot recurring weak points and regressions.
- **Validation:** `cargo test -p eddacraft-anvil -- adversarial_trends`
- **Dependencies:** ATC-003

## Execution

Action plan: [../execution/ATC.actions.md](../execution/ATC.actions.md)
