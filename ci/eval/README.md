# CI eval suites

First-wave trust-regression suites for the report-only policy eval-regression CI
check (EVALCI-005 / EVALCI-006).

## Layout

- `suites.json` — the suites manifest: an array of eval suites
  (`{ name, policy, query, input? }`) the `anvil policy eval-regression` command
  loads via `--suites`. Suites: the first-wave arch-boundary policy plus
  CPACKS-006 projections of the shipped `anvil-baseline` pack members
  (`change_scope` and `sensitive_paths`) with hermetic inputs under
  `ci/eval/inputs/` (not `policies/eval/`, so `opa test` does not merge
  PolicyInput JSON as data documents).
- `baseline/history.jsonl` — the committed one-record-per-suite baseline the CI
  check diffs each run against (`--store ci/eval/baseline`). Append-only NDJSON
  in Anvil's canonical eval-record schema.

## What the suites detect (EVALCI-010)

Every suite here evaluates a **committed, frozen** input, so its findings can
only change if the *policy* changed. Two verdicts are reported, and they are not
the same question:

| Verdict          | Reads                          | Fires when                                     |
| ---------------- | ------------------------------- | ---------------------------------------------- |
| `regressed`      | `exit_code` vs baseline         | The gate got worse. Unmoved by a rule going quiet — a resolved finding reads as an improvement, which is correct for a gate over changing code. |
| `output_changed` | `new_findings`/`resolved_findings` | The fixture's output moved **either way**, including a rule that stopped firing. Rendered `Δ` with an explicit callout. |

`output_changed` is the one that catches a policy silently breaking, and it is
why a changed fixture never renders as a clean `✓`. `regressed` keeps gate
semantics because the adversarial-probe runner depends on them.

The runner passes `--fail-on-warnings` to each `anvil policy eval` subprocess.
Without it an advisory suite reports `exit_code: 0` forever and `regressed`
cannot fire for it at all. Process exit 1 is an accepted verdict here, not a
runner failure.

**Bootstrap:** a first run may establish a baseline even though it reports as
regressed, but only when it is advisory-only. A first run carrying an
`error`-severity finding is still refused, so a broken suite cannot seed its own
baseline (EVALCI-001).

## CI behaviour

The `Policy eval-regression (report-only)` step in
`.github/workflows/rust-tests.yml` runs on every Rust-affecting PR and push. It
is `continue-on-error: true` and passes **no** `--fail-on-regression`, so it is
a non-blocking report during the report-only phase (ADR-002, warnings over
blocks). It never passes `--update-baseline`, so a CI run never mutates the
committed baseline.

## Refreshing the baseline (operator command)

The baseline is refreshed only by a deliberate operator action, never
automatically on every merge — an auto-update-on-merge would let a regressed run
silently become the accepted baseline (baseline poisoning; council 2026-07-01
decision 3). To regenerate after an intentional, reviewed change to a suite or
its policy:

```sh
cargo build -p eddacraft-anvil --bin anvil
ANVIL_DEV=1 target/debug/anvil policy eval-regression \
  --suites ci/eval/suites.json \
  --store ci/eval/baseline \
  --anvil-bin "$PWD/target/debug/anvil" \
  --update-baseline
```

`--update-baseline` appends a run to the history only when its gate did not
regress (the EVALCI-001 ratchet), so a failing run cannot poison the baseline.
Review and commit the resulting `baseline/history.jsonl` change on its own.
`ANVIL_DEV=1` skips the local licence pre-check so `anvil policy eval` runs
ungated.

The `anvil-baseline` eval suites are projections, not pack members. Keep their
thresholds and advisory copy in lockstep with
`crates/anvil-cli/src/commands/policy/starter_packs/anvil-baseline/policies/`.
`starter_policy_pack_change_scope_eval_wrapper_lockstep` and
`starter_policy_pack_sensitive_paths_eval_wrapper_lockstep` fail on drift **for
the fixtures they exercise** — today one per policy: the 12-file soft threshold
and one precise workflow-path match. The hard threshold and the name heuristics
are not covered, so drift confined to those branches passes. CPACKS-009 tracks
widening this.
