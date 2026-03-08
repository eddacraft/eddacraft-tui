# Adversarial Testing Catalog

| ID | Owner | Status |
|----|-------|--------|
| ATC | @aneki | Ready |

## Purpose

Build an Anvil-native catalog of adversarial test probes to continuously validate prompt safety, data handling, and model behavior regressions.

## In Scope

- Probe taxonomy and metadata model
- Reusable probe packs by risk category
- Probe execution hooks via eval harness integration
- Regression trend reporting for adversarial findings

## Tasks

### ATC-001: Define adversarial probe taxonomy
- **Intent:** Standardize categories, payload classes, and expected outcomes.
- **Expected Outcome:** Probe catalog supports traceable and versioned test assets.
- **Validation:** `pnpm nx test contracts --testNamePattern="adversarial taxonomy"`

### ATC-002: Implement probe pack registry
- **Intent:** Add loadable probe packs with versioned manifests.
- **Expected Outcome:** Probe sets can be selected by risk profile and context.
- **Validation:** `pnpm nx test core --testNamePattern="probe registry"`
- **Dependencies:** ATC-001

### ATC-003: Integrate probe execution into eval harness
- **Intent:** Execute adversarial probes in CI and local eval runs.
- **Expected Outcome:** Probe outcomes appear in eval regression summaries.
- **Validation:** `pnpm nx test core --testNamePattern="adversarial eval integration"`
- **Dependencies:** ATC-002, EVAL-002

### ATC-004: Add adversarial trend reporting
- **Intent:** Surface probe pass/fail trends by category over time.
- **Expected Outcome:** Teams can spot recurring weak points and regressions.
- **Validation:** `pnpm nx test cli --testNamePattern="adversarial trends"`
- **Dependencies:** ATC-003

## Execution

Steps: [../execution/ATC.steps.md](../execution/ATC.steps.md)
