# Actions: EVALCI

| Field  | Value                                                                          |
| ------ | ------------------------------------------------------------------------------ |
| Source | [../modules/eval-regression-ci-gate.aps.md](../modules/eval-regression-ci-gate.aps.md) |
| Task   | EVALCI — Full module execution                                                 |
| Status | In Progress                                                                    |

## Prerequisites

- [x] Module authorised (EVALCI-001..004 status flipped Proposed to Ready 2026-07-01)
- [x] eval-harness-integration available (EVAL Done; items Merged via PR #3013)
- [x] rust-tests.yml Test job has pinned OPA + Regal + rust-ci cache

## Actions

### 1. Ratchet baseline updates to clean runs

- **Checkpoint:** A failing run cannot be persisted as baseline.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_ratchet_baseline`
- **Status:** Merged 2026-07-01 via PR #3023 (EVALCI-001)

### 2. Null the eval subprocess stdin

- **Checkpoint:** A prompting suite fails fast, not at timeout.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_harness_adapter_subprocess_null_stdin`
- **Status:** Merged 2026-07-01 via PR #3023 (EVALCI-002)

### 3. Classify non-{0,1} suite exit as execution-error

- **Checkpoint:** Infra failures are errors, not regressions.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_harness_exit_code_classification`
- **Status:** Merged 2026-07-01 via PR #3023 (EVALCI-003)

### 4. Make per-suite failures fail-open

- **Checkpoint:** One broken suite still yields an aggregate report.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_command_fail_open`
- **Status:** Merged 2026-07-01 via PR #3023 (EVALCI-004)

### 5. Author the first-wave arch-boundary suite

- **Checkpoint:** A deterministic hermetic gating suite and manifest exist.
- **Validate:** `cargo test -p eddacraft-anvil-policy -- eval_suite_manifest_parses`
- **Status:** Merged 2026-07-04 via PR #3170 (EVALCI-005)

### 6. Wire report-only CI step with committed baseline

- **Checkpoint:** Every PR emits a non-blocking eval-regression report.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_command`
- **Status:** Merged 2026-07-04 via PR #3170 (EVALCI-006)

### 7. EVALCI-009 — Complete ADR-098 policy-support crate disposition

- **Checkpoint:** EXCEPT-012 has extracted exceptions; CLI-only eval,
  adversarial, and attack support has moved into the Rust CLI; unused policy
  config is deleted; `crates/anvil-policy` is no longer a workspace member.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_command`,
  `cargo test -p eddacraft-anvil -- attack_regression`,
  `cargo test -p eddacraft-anvil -- starter_policy_pack`, and
  `cargo check --workspace`
- **Status:** Proposed (EVALCI-009; required before EVALCI-008)

### 8. EVALCI-007 — Promote to visible non-required failure

- **Checkpoint:** Regressions surface as a visible check failure.
- **Validate:** workflow lint / dry-run of rust-tests.yml

### 9. EVALCI-008 — Promote to required hard-fail

- **Checkpoint:** A new trust regression blocks the PR.
- **Validate:** `cargo test -p eddacraft-anvil -- eval_regression_absorbs_new_violations_guard`

## Completion

- [ ] All checkpoints validated
- [ ] Work items marked Done in source module
