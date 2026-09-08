# CCTX-003 eval fixtures (2026-09-08)

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval artefacts) | 2026-09-08 — T06/T07 structural-partial-brief; recovery-cost field in scoring path; 20/20 `contract_ok` is authorship verification, not agent recall |

| Upstream | Downstream |
| -------- | ---------- |
| [Context Compiler spec](../../../specs/2026-09-08-context-compiler.md) §9 / §12 / §13 | [CCTX-003 comparison report](../../../audits/2026-09-08-cctx-003-baseline-comparison.md), [GCTX harness unblocker](../../../audits/2026-09-08-gctx-in-harness-unblocker.md), [CCTX module](../../../modules/context-compiler.aps.md) |

Internal evaluation artefacts for Context Compiler spike variants **V3** (brief
only) and **V4** (same brief plus GCTX drill-down). **Not** a product compiler,
knowledge store, crate, feature flag, or MCP tool.

A Decision Brief is **advisory**. It is never allow / warn / block.

**20/20 `contract_ok` is authorship verification** of these human-written
fixtures against frozen spec §9. It is **not** agent recall, not live V3
success, and not a V3-beats-V2 comparison. Live V3 was not run.

## Layout

| Path | Role |
| ---- | ---- |
| [`fixtures/T01.md`](fixtures/T01.md) … [`T20.md`](fixtures/T20.md) | One brief per corpus task |
| [`generate_fixtures.py`](generate_fixtures.py) | Authoring script (regenerates fixtures) |
| [`score_fixtures.py`](score_fixtures.py) | Mechanical §9 contract + gold-mention scorer; records §13 recovery cost as `unmeasured` on fixture runs |
| [`recovery_cost.py`](recovery_cost.py) | First-class §13 recovery-cost metric (`measured` / `none` / `unmeasured`) |
| [`score_live_session.py`](score_live_session.py) | Live-run scaffold: score a provided V1 session record; does not run sessions |
| [`test_eval_scoring.py`](test_eval_scoring.py) | Unit tests for recovery cost and T06/T07 structural-partial-brief |
| [`runs/2026-09-08-fixture-score.json`](runs/2026-09-08-fixture-score.json) | Dated score record |

V3 may open **only** paths listed under each brief's V3 drill-down. V4 may also
call spec §12.2 GCTX tools named in that brief. Gold paths outside the §12.3
allowlist are recall keys, not V3 selected inputs.

**T06 and T07** are **structural-partial-brief** cases: their gold evidence
sits outside spec §12.3, so those paths must stay recall keys and must not be
added as V3 handles. Live V3 cannot be scored as a sufficient brief-only run
for those tasks. Live V3 was not run.

Live V2/V4 remain **blocked** until the
[GCTX-in-harness unblocker](../../../audits/2026-09-08-gctx-in-harness-unblocker.md)
smoke checklist passes. Do not substitute V1.

## Re-score

From the repository root, with the corpus worktree at
`.worktrees/cctx-003-corpus` on `23457dc6d`:

```bash
python3 plans/evals/context-compiler/2026-09-08/score_fixtures.py
python3 -m unittest discover -s plans/evals/context-compiler/2026-09-08 -p 'test_*.py'
```

Live V1 sessions, when recorded, pass a session JSON to the scaffold (recovery
cost is `unmeasured` when events or tokens are missing — never silent zero):

```bash
python3 plans/evals/context-compiler/2026-09-08/score_live_session.py SESSION.json
```
