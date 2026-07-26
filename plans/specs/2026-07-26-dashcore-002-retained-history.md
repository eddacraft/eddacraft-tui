# DASHCORE-002 Retained Gate History and Trends

## Status

**Approved by the owner on 2026-07-26** via design grill.

Supersedes only the open “retained-history authority” gate on
[DASHCORE-002](../modules/dashboard-core-views.aps.md). Complements the Wave 2
honesty rules in
[2026-07-18-dashboard-core-wave-2-split.md](./2026-07-18-dashboard-core-wave-2-split.md)
(actual-range labelling, no padding, no invented samples).

## Goal

Expose **honest retained gate-health series** to the local browser dashboard so
Overview can render sparklines and trend charts without inventing points and
without depending on Kindling.

## Context

- `.anvil/gates.json` (`GateSnapshot`) is **latest-run only** and deliberately
  excludes multi-run history and timestamps suitable for time series.
- The dashboard server is loopback, workspace-scoped, and the only reader of
  `.anvil/` for browser UX (ADR-073 local runtime state).
- DASHCORE-001 current-state cards and later product routes must not claim
  30-day rates until a genuine series exists.

## Decisions

| # | Topic | Choice |
| - | ----- | ------ |
| 1 | Producer | Gate writer appends history when it persists a gate result |
| 2 | Storage | Append-only NDJSON under `.anvil/` (default path `.anvil/gate-history.ndjson`) |
| 3 | Series scope (v1) | Gate-derived fields only |
| 4 | Retention | 90 calendar days, then soft line cap (~500); drop oldest |
| 5 | API | Separate `GET /api/v1/protection/history` |
| 6 | Append failure | Best-effort; must not fail the gate write |
| 7 | Aggregation | Server returns raw ordered points; browser buckets daily/weekly |
| 8 | Bucket timezone | UTC (RFC 3339 timestamps) |

## Architecture

```text
anvil gate / TUI gate persist
  -> write .anvil/gates.json          (authoritative latest)
  -> best-effort append + GC          (.anvil/gate-history.ndjson)

anvil-dashboard-server
  -> read history artefact (capped)
  -> GET /api/v1/protection/history   (OpenAPI typed)

apps/dashboard
  -> TanStack Query history resource
  -> UTC day/week aggregation
  -> Recharts trends / sparklines
  -> actual-range labels (Wave 2 rules)
```

### Write path

1. After a successful write of the latest `GateSnapshot`, append one NDJSON line.
2. Minimum point fields (freeze names in OpenAPI during build). Align with
   the existing `GateSnapshot` contract where possible:
   - `recorded_at` — RFC 3339 UTC wall time of the append (history-only; not on
     `GateSnapshot`)
   - `score` — from `GateSnapshot.score`
   - `status` — from `GateSnapshot.status` (`pass` | `warn` | `fail`)
   - `status_label` — from `GateSnapshot.status_label` (display string)
   - `warning_count` — `GateSnapshot.warning_list.len()` (prefer list length over
     parsing the string `warnings` field)
   - optional but preferred when present: `duration_seconds`, `checks_run`
     (string forms as on `GateSnapshot`)
3. Run retention: drop points older than 90 days; if still over ~500 lines, drop
   oldest until under the cap.
4. On any history I/O or GC failure: log/diagnostic only; **gate command remains
   successful**. That run may be absent from history (honest shorter range).

### Read path

- New capability loads `.anvil/gate-history.ndjson` within the existing workspace
  artefact budget (`MAX_ARTEFACT_BYTES`).
- Response includes at least:
  - `schema_version` (e.g. `anvil.dashboard.protection-history.v1`)
  - `data_state`: `complete` | `partial` | `unavailable`
  - `source_message` and any `gaps` (including reserved drift/suppression gaps)
  - actual range: first/last `recorded_at` present (not a padded window)
  - ordered `points` array (chronological)
- Missing or empty file → `unavailable` (or empty series with explicit gap copy),
  **not** synthetic zeros.
- Drift and suppression series are **not** populated in v1; surface as named
  gaps / unavailable components so UI does not invent charts.

### UI

- Do not embed multi-week history on every `ProtectionOverview` poll.
- Charts and sparklines consume the history resource when the operator views
  Overview trend regions.
- Daily / weekly controls aggregate **in the browser** over raw points:
  - day/week boundaries: **UTC**
  - gate pass-rate for a bucket: fraction of points in that bucket whose
    `status` is `pass` (pin exact predicate in tests; `warn` is not a pass)
  - warning-count trend: aggregate warning counts per bucket (sum or last —
    prefer **last point in bucket** for “level” charts; document choice in
    implementation tests; default **last** to avoid double-counting multi-gate
    days as additive severity)
- Actual-range rules (binding from Wave 2 split):
  - use every genuine retained point available
  - target ≥ 30 days when present
  - show longer than 30 days when retained
  - show shorter ranges honestly and label covered dates
  - never pad missing days

## Non-goals

- Kindling / `gate.evaluated` as the v1 history backend
- Browser reads of `.anvil/`
- Multi-run full diagnostic archive (check trees per historical run)
- Drift or suppression time series producers in this item
- Failing gate on history write errors
- Invented or fixture-only production series

## Risks

| Risk | Mitigation |
| ---- | ---------- |
| High gate frequency hits line cap before 90 days | Labels use actual range; cap documented |
| Clock skew on `recorded_at` | UTC wall clock at write; no backdating |
| Partial file corruption | Fail closed to unavailable/partial + gap; do not invent points |
| Overview payload bloat | Separate history endpoint |

## Validation (design-level)

Implementation must prove:

- Append does not fail gate when history path is unwritable
- Retention drops aged and over-cap points deterministically
- History API reports honest `data_state` for missing/empty/corrupt/valid
- Client aggregation: UTC buckets, no padded days, short/long range labels
- Contract tests pin point ordering and source attribution
- Dashboard test/lint/typecheck/build and dashboard-server tests pass

## APS mapping

- Module: [dashboard-core-views](../modules/dashboard-core-views.aps.md)
- Work item: **DASHCORE-002**
- On Ready promotion: status `Proposed` → `Ready` only after this design and a
  `plan-ready` ReadyItem with exact validation commands

## Grill record

Owner answers (2026-07-26 design grill):

1. Producer → gate writer append  
2. Storage → NDJSON under `.anvil/`  
3. Series → gate-derived only  
4. Retention → 90 days + ~500 line cap  
5. API → `/api/v1/protection/history`  
6. Append failure → best-effort  
7. Aggregation → client from raw points  
8. Timezone → UTC  
