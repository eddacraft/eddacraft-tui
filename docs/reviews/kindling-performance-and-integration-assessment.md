# Kindling Performance and Integration Assessment

| Type  | Authority | Owner | Status | Freshness                                                                      |
| ----- | --------- | ----- | ------ | ------------------------------------------------------------------------------ |
| Guide | Advisory  | KFIT  | Draft  | Updated 2026-08-03 from local kindling KINTEG-013/014 work on base `c15089df2` |

| Upstream                                                                                                                                                                                                                                                                                                         | Downstream                                                     |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| [KFIT plan](../../plans/modules/kindling-product-fit.aps.md), [standard profile](../../plans/audits/2026-08-03-kindling-standard-profile.json), [stress profile](../../plans/audits/2026-08-03-kindling-stress-profile.json), [Criterion summary](../../plans/audits/2026-08-03-kindling-criterion-summary.json) | KFIT-005, KFIT-006, KFIT-007, KFIT-010, release/package review |

## Decision

The bundled runtime path is fast enough for the intended local integration. Keep
SQLite and FTS5, use the typed Rust runtime internally, and expose bounded
reads/status through anvil's existing MCP server only after the typed contract
is stable. Do not ingest observation history into the code graph, create a
second MCP server for the bundled path, or add anvil aggregation policy to
kindling.

The two measured implementation problems now have local fixes:

1. KINTEG-013 makes outage buffering independent of backlog depth, probes
   recovery with health before reading the spool, batches replay-status writes,
   and preserves configured byte/age retention with amortized low-water
   compaction.
2. KINTEG-014 adds transactional schema migration 006 and composite
   `(scope_id, ts, id)` indexes, removing SQLite's temporary sort from scoped
   keyset reads.

These changes are uncommitted and unpublished. They are review-ready evidence,
not an activated anvil dependency or release compatibility floor.

## Recorded evidence

| Artefact                                                                                                      | Purpose                                                 | State              |
| ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------ |
| [`benchmarks/history/kindling/2026-08-03.json`](../../benchmarks/history/kindling/2026-08-03.json)            | Normalised history row (workloads, budgets, verdicts)   | Filed              |
| [`docs/testing/benchmark-results.md`](../testing/benchmark-results.md) (Kindling section)                     | Human historical tables                                 | Filed              |
| [`2026-08-03-kindling-standard-profile.json`](../../plans/audits/2026-08-03-kindling-standard-profile.json)   | Release-mode 20k-row workload and resource profile      | Filed (+ raw copy) |
| [`2026-08-03-kindling-stress-profile.json`](../../plans/audits/2026-08-03-kindling-stress-profile.json)       | 200k-row reads and 100k-row outage/replay profile       | Filed (+ raw copy) |
| [`2026-08-03-kindling-criterion-summary.json`](../../plans/audits/2026-08-03-kindling-criterion-summary.json) | Thirteen Criterion cases and confidence intervals       | Filed (+ raw copy) |
| [`benchmarks/history/kindling/raw/`](../../benchmarks/history/kindling/raw/)                                  | Compact raw extracts (profiles, Criterion, query plans) | Filed              |
| `benchmark-results/manual-20260803T062924Z-kindling/`                                                         | Full HTML reports, source snapshot, diff                | Local, gitignored  |

The standard-profile SHA-256 is
`861e3fbfe23471b4a937670824c10e543f32b73ec58d5b51d43feaa34e936665`. The
stress-profile SHA-256 is
`0a5f1ef972cc6ada2aff6871568cae8b27e62bf2920153aec9dba0c0d1af76a7`.

## Results and budgets

| Workload                    | Standard result                                   | Stress/scale result                   | Assessment                                               |
| --------------------------- | ------------------------------------------------- | ------------------------------------- | -------------------------------------------------------- |
| Cold embedded startup       | p50 11.24 ms; p95 11.34 ms                        | p50 11.23 ms; p95 11.33 ms            | Healthy; proposed p95 budget 50 ms                       |
| Direct append               | p50 60.6 us; p95 111.8 us                         | p50 63.0 us; p95 123.2 us             | Healthy; proposed p95 budget 500 us                      |
| Warm daemon append          | p50 163 us; p95 230 us                            | p50 168 us; p95 237 us                | Healthy; proposed p95 budget 1 ms                        |
| Concurrent daemon append    | p50 817 us; p95 1.39 ms                           | p50 1.69 ms; p95 3.47 ms              | Healthy; proposed p95 budget 5 ms                        |
| Daemon page read            | p50 1.21 ms; p95 1.28 ms                          | p50 2.05 ms; p95 2.49 ms              | Healthy; proposed p95 budget 10 ms                       |
| Direct full list            | p50 42.9 ms over about 22.1k rows                 | p50 1.42 s over about 220.5k rows     | Linear; export/projection rebuild only                   |
| Daemon full list            | p50 94.9 ms over about 26.1k rows                 | p50 2.14 s over about 252.5k rows     | Linear; not an interactive query primitive               |
| Direct ranked retrieval     | p50 14.3 ms; p95 15.2 ms                          | p50 152.5 ms; p95 156.5 ms            | Healthy at 25k; realistic scale/selectivity work remains |
| Daemon ranked retrieval     | p50 17.3 ms; p95 19.2 ms                          | p50 178.3 ms; p95 185.7 ms            | Keep FTS5 and bounded results                            |
| Unbounded outage append     | p50 5.19 us; p95 6.49 us at 1k rows               | p50 5.16 us; p95 6.58 us at 100k rows | Healthy and flat; proposed p95 budget 5 ms               |
| Outage first/last windows   | p50 5.23/5.15 us                                  | p50 5.19/5.15 us for first/last 10k   | No positive backlog slope                                |
| 64 MiB capped outage append | Criterion fitted slope 36.2 us after crossing cap | Physical cap regression passes        | Healthy; amortized compaction preserves retention        |
| Replay                      | 1k in 168 ms; 5.96k rows/s                        | 100k in 18.3 s; 5.45k rows/s          | Healthy; proposed floor 2k rows/s                        |

At stress scale, the 28.7 MiB spool replay used about 65 MiB peak RSS above the
shared-process baseline. That is acceptable for evidence, but it is not a
release budget: workload groups still share one process. Isolated child-process
measurement is required before setting RSS/thread/FD release gates. Physical I/O
counters reported zero on this filesystem despite database growth, so WAL
checkpoint plus synchronized-filesystem measurement remains outstanding.

## Query and storage assessment

SQLite now plans the representative repository keyset list as
`SEARCH observations USING INDEX idx_obs_repo_ts` with no `TEMP B-TREE`.
Criterion measured the v6 10k-row full-list cases at about 12.8 ms direct and
27.6 ms through the daemon. The standard A/B improvement was roughly 145 ms to
43 ms direct and 253 ms to 95 ms through the daemon. Warm-write measurements
remained within budget, so the composite indexes earn their storage/write cost.

The list API is still linear when fully enumerated. Accepted kindling decision
D-009 excludes server-side aggregation: kindling remains mechanism, while
command/flag/principal semantics remain anvil policy. Interactive governance
should therefore use bounded repository/kind/time pages and an anvil-owned
incremental projection/cache. Exhaustive listing is retained for export,
diagnostics and projection rebuilds.

FTS5 remains the correct text-search mechanism. The code graph answers
structural questions about files, symbols and dependencies; kindling answers
historical questions about events and evidence. Join them at query time with
repository, file, symbol, gate-evaluation and trace identifiers. Do not
duplicate historical observations as graph nodes.

## Runtime and MCP boundary

The intended path is:

```text
anvil producers
  -> non-blocking typed sink
  -> one shared embedded kindling runtime
  -> SQLite/FTS5 plus durable spool

anvil CLI and existing MCP server
  -> bounded typed runtime reads/status

kindling evidence with file/symbol correlation
  -> optional bounded expansion through anvil graph context
```

Internal reads and writes should not use MCP. After KFIT-005 publishes a stable
runtime, anvil should start or attach once, route by canonical repository
identity, and shut down only a runtime it owns. The existing anvil MCP server
can later expose bounded evidence/history/status tools with repository scope,
limits, provenance, redaction and degraded-runtime semantics. A standalone
kindling MCP adapter may be useful to non-anvil consumers later, but it is not a
KFIT dependency.

## Required next sequence

1. ~~Merge kindling PR [#143](https://github.com/eddacraft/kindling/pull/143)~~
   **Done** (`f6dcd7d` on `main`).
2. ~~Post-merge isolated re-bench + history~~ **Done** for 2026-08-04:
   [`benchmarks/history/kindling/2026-08-04.json`](../../benchmarks/history/kindling/2026-08-04.json)
   (all budgeted workloads pass; concurrent stress p95 4.11 ms; isolated peak
   RSS stress outage ~174 MiB).
3. **Package + clean-consumer (KFIT-005 gate):** `cargo package --no-verify` OK
   for core crates. Still required under release authority: `--verify`, clean
   scratch-crate install, and a **0.4.0-or-newer** floor (workspace still
   0.3.0).
4. **KFIT-010 product work:** interactive surfaces must use **bounded
   repo/kind/time pages** only; full-list is export/rebuild. Implement
   anvil-owned incremental projection/cache for interactive counts — kindling
   must not grow server-side aggregation (D-009).
5. **Watch ranked retrieval scale:** standard (~25k) ~20 ms p95; stress ~180 ms
   needs selectivity/limit policy if interactive search at 100k+ rows is a
   claim.
6. **Watch concurrent daemon append** under real multi-producer anvil load
   (post-merge isolated stress p95 **4.11 ms** vs 5 ms budget — least write
   headroom).
7. Update anvil's dependency floor and activate KFIT-006 only after publication;
   then KFIT-007, KFIT-009, and MCP exposure of bounded reads.

There is no sound shortcut that activates the embedded runtime before upstream
publication: a path dependency on an unpublished kindling release would be
non-reproducible and would bypass the compatibility/package gate.
