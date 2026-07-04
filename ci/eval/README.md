# CI eval suites

First-wave trust-regression suites for the report-only policy eval-regression CI
check (EVALCI-005 / EVALCI-006).

## Layout

- `suites.json` — the suites manifest: an array of eval suites
  (`{ name, policy, query, input? }`) the `anvil policy eval-regression` command
  loads via `--suites`. First wave is one arch-boundary suite bound to
  `policies/eval/arch_boundary.rego` and its hermetic
  `arch_boundary.input.json`.
- `baseline/history.jsonl` — the committed one-record-per-suite baseline the CI
  check diffs each run against (`--store ci/eval/baseline`). Append-only NDJSON
  in Anvil's canonical eval-record schema.

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
