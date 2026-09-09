# GCTX token estimator Rust ↔ Python cross-check

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Audit | Advisory | CCTX / GCTX-020 | Complete | 2026-09-09 |

| Upstream | Downstream |
| -------- | ---------- |
| [2026-09-08 design council](../reviews/2026-09-08-cctx-design-council.md) (estimator fidelity follow-up) | Numeric §13 thresholds (blocked until this evidence); [CCTX-003 baseline](./2026-09-08-cctx-003-baseline-comparison.md) |

Internal evaluation only. **No product ships.** This note does **not** authorise
§13 numeric thresholds.

## Purpose

The 2026-09-08 design council required a Rust vs Python `estimate_gctx_tokens`
cross-check before any numeric §13 brief-size thresholds. Planning-budget
figures from the Python port must not be treated as crate-verified until this
passes.

## Implementations

| Side | Path |
| ---- | ---- |
| Rust (source of truth) | `crates/anvil-graph-cache/src/tokens.rs` (`gctx-simple-v1`, GCTX-020) |
| Python port | `plans/evals/context-compiler/2026-09-08/score_fixtures.py` |
| Driver | `plans/evals/context-compiler/2026-09-08/crosscheck_gctx_tokens.py` |
| Rust batch oracle | `crates/anvil-graph-cache/examples/estimate_gctx_tokens_batch.rs` |

## Behaviour checked

- Empty input → `0`
- ASCII prose, code-like identifiers, underscores
- Newlines / CRLF / tabs
- Non-ASCII (Latin, CJK, emoji, NBSP)
- Exact `MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES` (65536) accepted
- `MAX + 1` rejected (`InputTooLarge` / `GctxTokenEstimateError`)
- Real fixture briefs: T01, T05, T10, T15, T20

## Result

**21 PASS / 0 FAIL / 21 total.** Estimators match. No intentional divergences.

See machine record:

[`plans/audits/2026-09-09-gctx-token-estimator-xcheck.json`](./2026-09-09-gctx-token-estimator-xcheck.json)

| Metric | Value |
| ------ | ----- |
| PASS | 21 |
| FAIL | 0 |
| Intentional divergences | none |

### Port fix applied in this change

Before this change the Python port estimated oversized inputs instead of
rejecting them. It now raises `GctxTokenEstimateError` when
`input_bytes > MAX_GCTX_TOKEN_ESTIMATOR_INPUT_BYTES`, matching Rust
`TokenEstimateError::InputTooLarge`. Lexical classification parentheses were
clarified to `(ch.isascii() and ch.isalnum()) or ch == "_"` (same truth table
as before; matches `is_ascii_alphanumeric`).

## §13 thresholds

**Not invented here.** Estimator fidelity is a prerequisite; numeric brief-size
thresholds remain unset until a separate, evidence-backed decision.

## How to re-run

```bash
python3 plans/evals/context-compiler/2026-09-08/crosscheck_gctx_tokens.py
```
