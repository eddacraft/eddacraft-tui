# Benchmark history

Persistent, committed home for Anvil benchmark runs so perf data survives past
CI artifact retention (the `.github/workflows/bench.yml` /
`.github/workflows/bench-nightly.yml` workflows only `upload-artifact` with
`retention-days: 90`, then GitHub deletes them — this directory is the durable
record a trend graph can be built from).

## Layout

- `history/<YYYY-MM-DD>.json` — one curated run per file, schema below.
- `history/` is plain committed JSON (unlike the gitignored `benchmark-results/`
  scratch dir that raw `cargo bench` / `pnpm bench` output lands in).

## Schema (`schema_version: 1`)

```jsonc
{
  "schema_version": 1,
  "run": {
    "date", "commit", "rustc", "trigger", "host", "source",
    // optional, run-dependent:
    //   "anvil_version", "samples",
    //   "partial": true + "caveats": [...]  // for headline / non-comparable runs
  },
  "benches": {
    // bencher-format surfaces: array of { case, ns_per_iter, variance_ns }
    //   variance_ns may be null (older runs didn't capture it); an entry may
    //   carry an optional "note" for caveats (label drift, raw-vs-derived, …).
    "kernel": [...], "checks": [...], "stress": [...],
    "antipattern_scan": [...], "secret_scan_parallel": [...],
    "walk_discovery": [...],
    // harness=false latency gates: { case, samples, p50_ms, p95_ms, p99_ms }
    "ipc_roundtrip": [...], "hot_read": [...], "call_lift": [...],
    // criterion default surface: { case, median }
    "midedit_roundtrip": [...]
  },
  // resource-budget verdicts (from scripts/bench/run.sh when not skipped)
  "watch_resource_budget": { "status", "steady_state_cpu_pct", "peak_rss_mib", "budget" },
  "mcp_resource_budget": { "status", "steady_state_cpu_pct", "peak_rss_mib", "budget", "requests" },
  "intercept_resource_budget": { "idle": { ... }, "burst": { ... } },
  "concurrent_resource_budget": { "status", "steady_state_cpu_pct", "peak_rss_mib", "budget" }
}
```

## Adding a run

1. Run the criterion surfaces on a **quiet box** (criterion measures wall-clock;
   sibling builds corrupt the numbers). Either `pnpm bench` or, if that harness
   aborts in a non-TTY shell, the `cargo bench -p … --bench …` lines from
   `scripts/bench/run.sh` run directly.
2. Normalise the artifact dir into the schema above and save as
   `history/<date>.json`:

   ```bash
   python3 scripts/bench/to-history.py benchmark-results/manual-<timestamp> \
     --date YYYY-MM-DD
   ```

   Or hand-parse each bench's stdout. Keep `commit` + `host` accurate —
   comparisons across different hardware are not apples-to-apples.

## Comparing across runs

Compare runs from the **same hardware** — that is the axis that has to hold for
a delta to mean anything. Comparing across Anvil _versions_ on the same box is
the point: that is how a regression shows up. What breaks comparability is a
_different machine_: `2026-04-03.json` is a reconstructed marketing-era baseline
from another machine and the `0.3.0-beta` era, so its deltas vs
`2026-05-30.json` may reflect environment rather than code. Runs that are not
hardware-comparable carry `"partial": true` and a `caveats` list — treat their
cross-run direction as indicative, not a regression signal.

## Pre-`schema_version:1` data

Older runs predate this directory and were never committed (CI artifacts expire
at `retention-days: 90`). `2026-04-03.json` was reconstructed from a manually
saved vault export — headline cases only. If you find further historical
exports, drop them in `history/`: normalised to the schema above to join the
trend line, or as-is under a clearly-named file if the format differs.

Marketing collateral (pitch numbers, positioning) is intentionally **not** kept
here — this directory is raw benchmark data only.
