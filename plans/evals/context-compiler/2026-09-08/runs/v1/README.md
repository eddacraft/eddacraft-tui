# Live V1 run records

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval artefacts) | 2026-09-09 — T01–T05 (batch-A / CCTX-004), T06–T10 (batch-B), T11–T15 (batch-C), and T16–T20 (batch-D) live V1 sessions recorded |

| Upstream | Downstream |
| -------- | ---------- |
| [V1 runbook](../../V1-RUNBOOK.md), [residual map](../../residual-map.md), [eval README](../../README.md) | Live V1 session JSON / optional transcripts scored by [`score_v1.py`](../../score_v1.py) |

Dated §12.7 ordinary-exploration sessions. Corpus
`23457dc6d2bf379791d587cf2dfdb5046ce51fc0`.

| Path | Role |
| ---- | ---- |
| `Txx.json` | One independent V1 session |
| `Txx.md` | Optional transcript notes |
| `batch-t01-t05.json` | Batch-A roll-up for T01–T05 (CCTX-004 / this PR) |
| `T01.json` … `T05.json` | Batch-A live V1 session records |
| `batch-t06-t10.json` | Batch-B roll-up for T06–T10 |
| `T06.json` … `T10.json` | Batch-B live V1 session records |
| `batch-t11-t15.json` | Batch-C roll-up for T11–T15 |
| `T11.json` … `T15.json` | Batch-C live V1 session records |
| `batch-t16-t20.json` | Batch-D roll-up for T16–T20 |
| `T16.json` … `T20.json` | Batch-D live V1 session records |
| `score-rollup.json` | Mechanical `score_v1.py` output |
