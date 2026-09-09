# Live V1 run records

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval artefacts) | 2026-09-08 — directory scaffold for live §12.7 V1 sessions; no session records yet |

| Upstream | Downstream |
| -------- | ---------- |
| [V1 runbook](../../V1-RUNBOOK.md), [residual map](../../residual-map.md), [eval README](../../README.md) | Live V1 session JSON / optional transcripts scored by [`score_v1.py`](../../score_v1.py) |

Dated §12.7 ordinary-exploration sessions. Corpus
`23457dc6d2bf379791d587cf2dfdb5046ce51fc0`.

| Path | Role |
| ---- | ---- |
| `Txx.json` | One independent V1 session |
| `Txx.md` | Optional transcript notes |
| `batch-t01-t05.json` | First-batch roll-up — created when T01–T05 sessions are recorded (not in this PR yet) |
| `score-rollup.json` | Mechanical `score_v1.py` output |
