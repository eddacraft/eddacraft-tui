# Steps: EVALCI

| Field  | Value                                                                          |
| ------ | ------------------------------------------------------------------------------ |
| Source | [../modules/eval-regression-ci-gate.aps.md](../modules/eval-regression-ci-gate.aps.md) |
| Task   | EVALCI — Full module execution                                                 |
| Status | Proposed                                                                       |

## Prerequisites

- [ ] Module authorised (status flipped from Proposed to Ready)
- [ ] eval-harness-integration available (EVAL Done; items Merged via PR #3013)
- [ ] rust-tests.yml Test job has pinned OPA + Regal + rust-ci cache

## Steps

### 1. Ratchet baseline updates to clean runs

- **Checkpoint:** A failing run cannot be persisted as baseline.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_ratchet_baseline`

### 2. Null the eval subprocess stdin

- **Checkpoint:** A prompting suite fails fast, not at timeout.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_harness_adapter_subprocess_null_stdin`

### 3. Classify non-{0,1} suite exit as execution-error

- **Checkpoint:** Infra failures are errors, not regressions.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_harness_exit_code_classification`

### 4. Make per-suite failures fail-open

- **Checkpoint:** One broken suite still yields an aggregate report.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_command_fail_open`

### 5. Author the first-wave arch-boundary suite

- **Checkpoint:** A deterministic hermetic gating suite and manifest exist.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_suite_manifest_parses`

### 6. Wire report-only CI step with committed baseline

- **Checkpoint:** Every PR emits a non-blocking eval-regression report.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_command`

### 7. Promote to visible non-required failure

- **Checkpoint:** Regressions surface as a visible check failure.
- **Validate:** workflow lint / dry-run of rust-tests.yml

### 8. Promote to required hard-fail

- **Checkpoint:** A new trust regression blocks the PR.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_regression_absorbs_new_violations_guard`

## Completion

- [ ] All checkpoints validated
- [ ] Work items marked Done in source module
