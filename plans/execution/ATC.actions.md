# Actions: ATC

| Field  | Value                                                                                            |
| ------ | ------------------------------------------------------------------------------------------------ |
| Source | [../modules/adversarial-testing-catalog.aps.md](../modules/adversarial-testing-catalog.aps.md)   |
| Task   | ATC — Full module execution                                                                      |
| Status | In Progress                                                                                       |

## Prerequisites

- [x] Adversarial probe categories and expectations agreed (ATC-001 taxonomy)
- [x] Eval harness integration available (EVAL Done; EVALCI suites landed —
      `policies/eval/` + `ci/eval/suites.json`)

## Actions

### 1. Define adversarial taxonomy (ATC-001)

- **Checkpoint:** Probe categories, payload classes, and expected outcomes are
  versioned wire types with `#[serde(other)]` forward-compat fallbacks.
- **Validate:** `cargo test -p eddacraft-anvil-kernel-types -- adversarial_taxonomy`

### 2. Build probe pack registry (ATC-002)

- **Checkpoint:** Versioned probe packs load fail-closed and are selectable by
  risk profile, with deterministic ordering and path containment.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- probe_registry`

### 3. Integrate probe execution into the eval harness (ATC-003)

- **Checkpoint:** Probe outcomes fold into the frozen `EvalRunSummary` shape and
  appear alongside policy suites without changing the eval `--json` v1 contract.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- adversarial_eval_integration`

### 4. Add adversarial trend reporting (ATC-004)

- **Checkpoint:** Probe pass/fail trends by category over time are read from the
  eval store history through a tested library function.
- **Validate:** `cargo test -p eddacraft-anvil -- adversarial_trends`

## Completion

- [ ] All checkpoints validated
- [ ] Work items marked Done in source module
