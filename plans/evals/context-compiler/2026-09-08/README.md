# CCTX-003 eval fixtures (2026-09-08)

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Advisory | CCTX | Draft (eval artefacts) | 2026-09-08 — V3/V4 briefs authored against frozen spec §9 at corpus `23457dc6d2bf379791d587cf2dfdb5046ce51fc0` |

| Upstream | Downstream |
| -------- | ---------- |
| [Context Compiler spec](../../specs/2026-09-08-context-compiler.md) §9 / §12 / §13 | [CCTX-003 comparison report](../../audits/2026-09-08-cctx-003-baseline-comparison.md), [CCTX module](../../modules/context-compiler.aps.md) |

Internal evaluation artefacts for Context Compiler spike variants **V3** (brief
only) and **V4** (same brief plus GCTX drill-down). **Not** a product compiler,
knowledge store, crate, feature flag, or MCP tool.

A Decision Brief is **advisory**. It is never allow / warn / block.

## Layout

| Path | Role |
| ---- | ---- |
| [`fixtures/T01.md`](fixtures/T01.md) … [`T20.md`](fixtures/T20.md) | One brief per corpus task |
| [`generate_fixtures.py`](generate_fixtures.py) | Authoring script (regenerates fixtures) |
| [`score_fixtures.py`](score_fixtures.py) | Mechanical §9 contract + gold-mention scorer |
| [`runs/2026-09-08-fixture-score.json`](runs/2026-09-08-fixture-score.json) | Dated score record |

V3 may open **only** paths listed under each brief's V3 drill-down. V4 may also
call spec §12.2 GCTX tools named in that brief. Gold paths outside the §12.3
allowlist are recall keys, not V3 selected inputs.

## Re-score

From the repository root, with the corpus worktree at
`.worktrees/cctx-003-corpus` on `23457dc6d`:

```bash
python3 plans/evals/context-compiler/2026-09-08/score_fixtures.py
```
