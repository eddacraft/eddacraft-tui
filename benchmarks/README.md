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
    "antipattern_catalogue": {
      "schema_version",
      "fingerprint",             // SHA-256 over performance-relevant rule fields
      "enabled_scanner_fingerprint",
      "pattern_count",
      "scanner_pattern_count",
      "enabled_scanner_pattern_count",
      "default_scanner_pattern_count",
      "default_source_rules": { "typescript", "rust", "python" }
    },
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

   The normaliser reads `patterns/compiled/registry.json` by default and stamps
   its performance fingerprint and per-language default-rule counts. Pass
   `--registry` when the benchmarked run used a registry outside that checkout.
   Or hand-parse each bench's stdout. Keep `commit`, `host`, and
   `antipattern_catalogue` accurate — comparisons across different hardware or
   rule workloads are not apples-to-apples.

## Comparing across runs

Compare runs from the **same hardware** — that is the axis that has to hold for
a delta to mean anything. Comparing across Anvil _versions_ on the same box is
the point: that is how a regression shows up. What breaks comparability is a
_different machine_: `2026-04-03.json` is a reconstructed marketing-era baseline
from another machine and the `0.3.0-beta` era, so its deltas vs
`2026-05-30.json` may reflect environment rather than code. Runs that are not
hardware-comparable carry `"partial": true` and a `caveats` list — treat their
cross-run direction as indicative, not a regression signal.

For antipattern results, also require the same benchmark case and applicable
catalogue fingerprint. The mixed-corpus case contains both a semantic version
and a deterministic content digest, so fixture-content changes automatically
create a new case identity. Default scanner cases use
`run.antipattern_catalogue.fingerprint`; the `html_opt_in` case uses
`enabled_scanner_fingerprint`, which also covers enabled opt-in regex rules. A
changed case or fingerprint establishes a new baseline: report the workload
delta separately instead of classifying the timing delta as a scanner
regression. Both fingerprints exclude generated timestamps, prose-only fields,
and AST rules the scanner does not execute, but include matching, target,
extension, and allowlist fields that can change cost.

## Pre-`schema_version:1` data

Older runs predate this directory and were never committed (CI artifacts expire
at `retention-days: 90`). `2026-04-03.json` was reconstructed from a manually
saved vault export — headline cases only. If you find further historical
exports, drop them in `history/`: normalised to the schema above to join the
trend line, or as-is under a clearly-named file if the format differs.

Marketing collateral (pitch numbers, positioning) is intentionally **not** kept
here — this directory is raw benchmark data only.

## Kindling (consumer-side adoption evidence)

Kindling performance numbers that decide **anvil** adoption, packaging, or KFIT
gates live **here in anvil**, not in the public kindling repository. The public
product must not carry anvil-specific benchmark dumps or marketing tables.

### Layout

- `history/kindling/<YYYY-MM-DD>.json` — normalised run: host metadata, every
  workload p50/p95/p99, proposed budgets, verdicts, and optional Criterion
  summary.
- `history/kindling/raw/` — compact source artefacts kept for forensics
  (standard/stress profile JSON, Criterion summary, query plans, status). Full
  Criterion HTML/SVG trees stay under gitignored `benchmark-results/`.

### Schema (`schema_version: 1`, `suite: "kindling"`)

```jsonc
{
  "schema_version": 1,
  "suite": "kindling",
  "run": {
    "date", "kindling_commit", "anvil_commit", "trigger", "host", "source",
    // optional: "partial": true, "caveats": [...]
  },
  "profiles": { "standard": { "config", ... }, "stress": { ... } },
  "workloads": [
    {
      "id": "daemon/list-page",
      "profile": "standard",
      "p50_us", "p95_us", "p95_ms", "rows_processed", "verdict", "budget"
    }
  ],
  "resources": [ { "profile", "group", "peak_rss_mib", "spool_bytes", ... } ],
  "criterion": { "benchmarks": [ { "benchmark", "mean_ns", "median_ns", ... } ] }
}
```

### Adding a kindling run

1. Run the kindling harness on a quiet box (release build). Drop the scratch
   tree under `benchmark-results/manual-<timestamp>-kindling/` (gitignored).
2. Promote compact profiles into history:

   ```bash
   python3 scripts/bench/kindling-to-history.py \
     --date YYYY-MM-DD \
     --standard path/to/kindling-perf.json \
     --stress path/to/kindling-stress.json \
     --criterion path/to/criterion-summary.json \
     --kindling-commit <sha> \
     --source "benchmark-results/manual-<timestamp>-kindling"
   ```

3. Optionally copy the compact JSON/text extracts into `history/kindling/raw/`
   (do **not** force-add Criterion HTML reports).
4. Append a dated table to
   [`docs/testing/benchmark-results.md`](../docs/testing/benchmark-results.md)
   (Kindling section).

### Comparability

Same rules as anvil benches: compare runs on the **same host class**. Cross-host
deltas are indicative only. Uncommitted kindling worktrees and shared-process
RSS figures must carry `partial` + `caveats`.
