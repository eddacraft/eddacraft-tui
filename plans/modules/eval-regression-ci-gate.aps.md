# Eval Regression CI Gate

| ID     | Owner  | Status   |
| ------ | ------ | -------- |
| EVALCI | @aneki | Proposed |

**Last reviewed:** 2026-07-01 (planning council: architect + pragmatic-lead + adversarial-reviewer)

## Purpose

`anvil policy eval-regression` shipped report-only as part of the
eval-harness-integration module (EVAL, Done — EVAL-001..005 Merged 2026-06-30
via PR #3013). This module wires that command into CI as a
trust-regression gate: a check that fails when a change makes policy evaluation
regress against a committed baseline. A planning council (architect,
pragmatic-lead, adversarial-reviewer) reviewed the direction on 2026-07-01,
settled four decisions, and surfaced four code prerequisites that must land
before a *blocking* gate is safe — otherwise the gate can produce false
negatives (baseline poisoning, absorbed violations, suppressed runner errors)
and quietly stop catching regressions. Direction is reviewed but execution is
not yet authorised: this module and every work item below are `Proposed`. See
the post-merge note
[`../reviews/post-merge/feat-eval-harness-integration.md`](../reviews/post-merge/feat-eval-harness-integration.md)
for the deferred wiring step this module picks up.

## In Scope

- Hardening the eval-regression command so a blocking gate cannot produce false
  negatives — baseline poisoning, absorbed violations, and runner-error
  suppression
- A first-wave committed eval suite plus manifest and a committed baseline
- Report-only CI wiring, then phased promotion to a required blocking check

## Out of Scope

- Authoring ATC/PATT suite depth — probe and prompt-attack content belong to
  the adversarial-testing-catalog (ATC) and prompt-attack-regression-packs
  (PATT) modules
- A net-new eval framework — this module wires the existing
  `anvil policy eval-regression` command, it does not replace it

## Council Decisions (2026-07-01)

1. **Suites:** committed `policies/eval/*.rego` plus hermetic `*.input.json`
   fixtures, indexed by a `ci/eval/suites.json` manifest; first wave is one
   arch-boundary suite (`data.anvil.arch.findings`). No workspace-scanning
   suites in a blocking gate.
2. **Posture:** report-only default stays (honours ADR-002, which governs the
   product UX; CI opting into `--fail-on-regression` is an operator choice);
   phase annotate then non-required failure then required hard-fail.
3. **Baseline:** committed one-record-per-suite under `ci/eval/baseline/`,
   read-only on PRs, `--update-baseline` only on main after merge (never
   automatically on every merge — that is baseline poisoning). Alternative
   `actions/cache` rejected: a cold cache cold-start would spuriously block.
4. **Runner:** graft a step onto the `.github/workflows/rust-tests.yml` Test job
   (already has pinned OPA plus Regal and the rust-ci cache); build anvil and
   pass its absolute path via `--anvil-bin` (CI `current_exe()` resolves to the
   test binary, not the product CLI).

## Work Items

### EVALCI-001: Ratchet `--update-baseline` to clean runs only

- **Status:** Proposed
- **Intent:** Persist a run to history only when its gate is non-regressed or
  clean, so a failing run cannot become the accepted baseline.
- **Expected Outcome:** `--update-baseline` cannot poison the baseline with a
  failing run.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_ratchet_baseline`
- **Dependencies:** none

### EVALCI-002: Null the eval subprocess stdin

- **Status:** Proposed
- **Intent:** `SubprocessRunner` sets child stdin to null so a future
  auth or license prompt cannot hang a suite until the timeout.
- **Expected Outcome:** A prompting upstream command fails fast, not after the
  60s budget.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_harness_adapter_subprocess_null_stdin`
- **Dependencies:** none

### EVALCI-003: Classify non-{0,1} suite exit as execution-error

- **Status:** Proposed
- **Intent:** An inner `anvil policy eval` exit code outside {0,1} (an OPA or
  infra failure, for example 2) is an execution error, not a trust regression.
- **Expected Outcome:** Infra failures do not false-block main as regressions.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_harness_exit_code_classification`
- **Dependencies:** none

### EVALCI-004: Per-suite fail-open

- **Status:** Proposed
- **Intent:** A suite that errors (missing policy, empty stdout) reports a
  `runner-error` status and the run continues and still emits the aggregate
  report, instead of aborting all suites.
- **Expected Outcome:** One broken suite cannot suppress regression detection
  for the rest.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command_fail_open`
- **Dependencies:** none

### EVALCI-005: First-wave arch-boundary eval suite

- **Status:** Proposed
- **Intent:** Author `policies/eval/arch_boundary.rego` plus a hermetic
  `arch_boundary.input.json` and `ci/eval/suites.json`; extend `opa test` and
  `regal lint` to cover `policies/eval/`.
- **Expected Outcome:** A deterministic, self-contained gating suite exists.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_suite_manifest_parses`
  (plus `opa test policies/eval/`)
- **Dependencies:** EVALCI-002, EVALCI-003

### EVALCI-006: Report-only CI step plus committed baseline

- **Status:** Proposed
- **Intent:** Add a `continue-on-error: true` eval-regression step to the
  rust-tests.yml Test job (build anvil, absolute `--anvil-bin`,
  `--store ci/eval/baseline`), and seed a one-record-per-suite baseline written
  only on main push.
- **Expected Outcome:** Every PR gets a non-blocking eval-regression report.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command`
  (command contract) plus workflow lints
- **Dependencies:** EVALCI-001, EVALCI-004, EVALCI-005

### EVALCI-007: Phase-2 promotion to visible non-required failure

- **Status:** Proposed
- **Intent:** After a burn-in on main, drop `continue-on-error` so regressions
  surface as a visible (still non-required) check failure.
- **Expected Outcome:** Regressions are visible without blocking merges.
- **Validation:** workflow lint / dry-run
- **Dependencies:** EVALCI-006

### EVALCI-008: Phase-3 required hard-fail

- **Status:** Proposed
- **Intent:** Add `--fail-on-regression` and make the check required; gated on
  an ADR for the CI-blocking posture (ADR-002 reconciliation) and on resolving
  the "already-failing gate absorbs new violations" design question (flag new
  blocking findings even when the exit code stays 1 to 1).
- **Expected Outcome:** A new trust regression blocks the PR.
- **Validation:** workflow lint plus
  `cargo test -p eddacraft-anvil-policy -- eval_regression_absorbs_new_violations_guard`
- **Dependencies:** EVALCI-007, ATC-003

## Execution

Action plan: [../execution/EVALCI.actions.md](../execution/EVALCI.actions.md)
