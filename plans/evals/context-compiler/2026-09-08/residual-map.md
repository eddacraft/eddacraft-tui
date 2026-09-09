# CCTX live V1 residual map

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ------ | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval coordination) | 2026-09-09 — T06–T10 recorded by eval/cctx-v1-batch-B (this PR); T11–T15 recorded by eval/cctx-v1-batch-C; T16–T20 recorded by eval/cctx-v1-batch-D; T01–T05 remain claimed by CCTX-004 / #4466 |

| Upstream | Downstream |
| -------- | ---------- |
| [V1 runbook](V1-RUNBOOK.md), spec §12.6 task corpus | Parallel V1 siblings; CCTX-004 claim [#4466](https://github.com/eddacraft/anvil-001/issues/4466) |

Coordination only. Not a product backlog. Spec §15 stays parked. V2/V4 stay
**blocked** until GCTX is in the harness — blocked ≠ V1.

## How to take a batch

1. Read this table on `main` and on open CCTX V1 PRs.
2. Pick a contiguous **available** block (prefer five tasks).
3. Edit this file on your branch: set those rows to `claimed` with your PR or
   issue, before you dispatch sessions.
4. After durable `runs/v1/Txx.json` records exist, set those rows to `recorded`.

Do not claim all twenty unfinished. A partial recorded batch plus a clear
residual is the done bar for a first PR.

## Task board

| ID | Class | Live V1 | Owner | Notes |
| -- | ----- | ------- | ----- | ----- |
| T01 | orientation | claimed | CCTX-004 #4466 | |
| T02 | orientation | claimed | CCTX-004 #4466 | |
| T03 | orientation | claimed | CCTX-004 #4466 | |
| T04 | localised bug | claimed | CCTX-004 #4466 | GATT / `ImpactSummary.truncated` |
| T05 | localised bug | claimed | CCTX-004 #4466 | CALL-1 heuristic OR-across-edges |
| T06 | localised bug | recorded | eval/cctx-v1-batch-B (this PR; T06–T10) | Sibling batch candidate |
| T07 | localised bug | recorded | eval/cctx-v1-batch-B (this PR; T06–T10) | Sibling batch candidate |
| T08 | cross-module change | recorded | eval/cctx-v1-batch-B (this PR; T06–T10) | Blast-radius scoring applies |
| T09 | cross-module change | recorded | eval/cctx-v1-batch-B (this PR; T06–T10) | Blast-radius scoring applies |
| T10 | cross-module change | recorded | eval/cctx-v1-batch-B (this PR; T06–T10) | Blast-radius scoring applies |
| T11 | cross-module change | recorded | eval/cctx-v1-batch-C (T11–T15) | Blast-radius scoring applies |
| T12 | policy-sensitive change | recorded | eval/cctx-v1-batch-C (T11–T15) | |
| T13 | policy-sensitive change | recorded | eval/cctx-v1-batch-C (T11–T15) | **High priority** — live leakage score (council should_fix). Keep `anvil_validate_write`. |
| T14 | policy-sensitive change | recorded | eval/cctx-v1-batch-C (T11–T15) | EVALCI vs CCTX |
| T15 | policy-sensitive change | recorded | eval/cctx-v1-batch-C (T11–T15) | Honest **uncertainty**; do not invent an ADR (§15.7) |
| T16 | test-impact | recorded | eval/cctx-v1-batch-D (T16–T20) | Blast-radius scoring applies |
| T17 | test-impact | recorded | eval/cctx-v1-batch-D (T16–T20) | Blast-radius scoring applies |
| T18 | test-impact | recorded | eval/cctx-v1-batch-D (T16–T20) | Blast-radius scoring applies |
| T19 | novel structural question | recorded | eval/cctx-v1-batch-D (T16–T20) | Honest **parked** (§15.6); do not fork GATT |
| T20 | novel structural question | recorded | eval/cctx-v1-batch-D (T16–T20) | Honest **no** — affinity, not membership |

Suggested sibling slices: T06–T10, T11–T15 (includes T13), T16–T20.

## Other parallel tracks (not this board)

These are **not** live V1 session claims. Do not treat them as owning T-IDs:

- GCTX harness unblocker — V2/V4 only, once tools exist
- Token-estimator Rust vs Python check — before numeric §13 thresholds
- T06/T07 V3 handle adequacy — before live V3, not V1

## Non-scope (still parked / blocked)

- Spec §15, including §15.6
- Product compiler
- V2/V4 live sessions while GCTX is unavailable
