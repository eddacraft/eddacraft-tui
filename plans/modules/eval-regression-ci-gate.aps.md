# Eval Regression CI Gate

| ID     | Owner  | Status   |
| ------ | ------ | -------- |
| EVALCI | @aneki | In Progress |

**Last reviewed:** 2026-07-11 (post-POLRESET downstream coherence review —
`plans/reviews/2026-07-11-polreset-downstream-coherence.md`)

> First-wave hardening EVALCI-001..004 authorised **Ready** 2026-07-01 (owner)
> and since delivered. **EVALCI-005/006 Merged 2026-07-04 via PR #3170**
> (= POLRESET-008: every Rust-affecting PR now gets the non-blocking
> `Policy eval-regression (report-only)` step). 007/008 remain **Proposed**:
> 007 waits on a burn-in on main; 008's ATC-003 dependency is satisfied (ATC
> Done, Merged 2026-07-05 via PR #3181), leaving the CI-blocking-posture ADR
> (POLRESET design gate 3) as its **sole remaining decision gate** — no work
> item anywhere tracks authoring that ADR; it is a deliberate operator
> decision, not queued code work.

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
and quietly stop catching regressions. The four code prerequisites
(EVALCI-001..004) were authorised `Ready` (2026-07-01) and delivered; the
first-wave suite and report-only CI wiring (EVALCI-005/006) merged 2026-07-04
via PR #3170. Only the phased promotion (EVALCI-007/008) remains `Proposed`,
pending a burn-in and the CI-blocking-posture ADR. See the post-merge note
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

- **Status:** Merged 2026-07-01 via PR #3023
- **Intent:** Persist a run to history only when its gate is non-regressed or
  clean, so a failing run cannot become the accepted baseline.
- **Expected Outcome:** `--update-baseline` cannot poison the baseline with a
  failing run.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_ratchet_baseline`
- **Dependencies:** none

### EVALCI-002: Null the eval subprocess stdin

- **Status:** Merged 2026-07-01 via PR #3023
- **Intent:** `SubprocessRunner` sets child stdin to null so a future
  auth or license prompt cannot hang a suite until the timeout.
- **Expected Outcome:** A prompting upstream command fails fast, not after the
  60s budget.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_harness_adapter_subprocess_null_stdin`
- **Dependencies:** none

### EVALCI-003: Classify non-{0,1} suite exit as execution-error

- **Status:** Merged 2026-07-01 via PR #3023
- **Intent:** An inner `anvil policy eval` exit code outside {0,1} (an OPA or
  infra failure, for example 2) is an execution error, not a trust regression.
- **Expected Outcome:** Infra failures do not false-block main as regressions.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_harness_exit_code_classification`
- **Dependencies:** none

### EVALCI-004: Per-suite fail-open

- **Status:** Merged 2026-07-01 via PR #3023
- **Intent:** A suite that errors (missing policy, empty stdout) reports a
  `runner-error` status and the run continues and still emits the aggregate
  report, instead of aborting all suites.
- **Expected Outcome:** One broken suite cannot suppress regression detection
  for the rest.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command_fail_open`
- **Dependencies:** none

### EVALCI-005: First-wave arch-boundary eval suite

- **Status:** Merged 2026-07-04 via PR #3170
- **Intent:** Author `policies/eval/arch_boundary.rego` plus a hermetic
  `arch_boundary.input.json` and `ci/eval/suites.json`; extend `opa test` and
  `regal lint` to cover `policies/eval/`.
- **Expected Outcome:** A deterministic, self-contained gating suite exists.
- **Validation:** `cargo test -p eddacraft-anvil-policy -- eval_suite_manifest_parses`
  (plus `opa test policies/eval/`)
- **Dependencies:** EVALCI-002, EVALCI-003
- **Validation notes:** `policies/eval/arch_boundary.rego` (package
  `anvil.policies.arch_boundary`, findings over `input.diff.new_edges` — new
  edges only, ADR-003; crossing edge is `warning`, ADR-002) plus
  `arch_boundary_test.rego` (positive/negative/baseline-not-reflagged/threshold)
  and hermetic `arch_boundary.input.json`; manifest `ci/eval/suites.json` binds
  the frozen `EvalSuite` shape (one arch-boundary suite, query
  `data.anvil.policies.arch_boundary.findings`). `opa test policies/eval/`
  green (4/4); `cargo test -p eddacraft-anvil-policy -- eval_suite_manifest_parses`
  green (added to `crates/anvil-policy/src/eval/port.rs`, binds the committed
  manifest). `opa test`/`regal lint` extended to `policies/eval/` in `ci.yml`
  and `rust-tests.yml` (the `policies/**/*` classifier already triggers
  opa-test/regal on eval-policy changes). Regal not installed locally — CI runs
  the pinned v0.41.1.

### EVALCI-006: Report-only CI step plus committed baseline

- **Status:** Merged 2026-07-04 via PR #3170
- **Intent:** Add a `continue-on-error: true` eval-regression step to the
  rust-tests.yml Test job (build anvil, absolute `--anvil-bin`,
  `--store ci/eval/baseline`), and seed a one-record-per-suite baseline written
  only on main push.
- **Expected Outcome:** Every PR gets a non-blocking eval-regression report.
- **Validation:** `cargo test -p eddacraft-anvil -- eval_regression_command`
  (command contract) plus workflow lints
- **Dependencies:** EVALCI-001, EVALCI-004, EVALCI-005
- **Validation notes:** `Policy eval-regression (report-only)` step added to the
  `rust-tests.yml` Test job (after the tui gates): `continue-on-error: true`,
  builds `cargo build -p eddacraft-anvil --bin anvil` (debug, cheapest — no
  guaranteed pre-existing artefact) and runs `anvil policy eval-regression`
  with absolute `--anvil-bin "$PWD/target/debug/anvil"`,
  `--suites ci/eval/suites.json`, `--store ci/eval/baseline`; no
  `--fail-on-regression` (report-only, ADR-002) and no `--update-baseline`
  (never mutates the baseline from CI). `ANVIL_DEV=1` bypasses the local
  licence pre-check so the subprocess `anvil policy eval` runs ungated (`policy`
  is in `CLI_GATED_COMMANDS`). Report goes to the job log and step summary.
  **Baseline mechanism:** committed `ci/eval/baseline/history.jsonl` (one
  deterministic-seed record per suite), read-only compare on PR/push; refresh
  is a documented operator command (`ci/eval/README.md`), not an automatic
  main-push commit — CI cannot push to main and council decision 3 forbids
  auto-update-on-every-merge (baseline poisoning), so the ambiguous
  "written on main push" is resolved to the operator-refresh path. Existing
  `eval_regression_command` tests (10) stay green; workflow yaml passes
  `oxfmt --check`.

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
- **Dependencies:** EVALCI-007; ATC-003 (satisfied — Merged 2026-07-05 via
  PR #3181); CI-blocking-posture ADR (POLRESET design gate 3, not yet authored)

## Execution

Action plan: [../execution/EVALCI.actions.md](../execution/EVALCI.actions.md)
