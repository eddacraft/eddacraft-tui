# DEVACC MVP evidence note (2026-08-11)

| Field                  | Value                                                   |
| ---------------------- | ------------------------------------------------------- |
| Branch                 | `feat/devacc-complete`                                  |
| Suite                  | Tier A deterministic + Tier B dry-run scaffold          |
| Publishable hero claim | **No** — live Tier B n≥10 / SCN-40 not run with a model |

## Tier A (reproducible)

Command:

```bash
cargo test -p anvil-bench --lib -- devacc_
cargo run -p anvil-bench --bin anvil-bench -- devacc --tier A
```

Sample paired reductions (`gctx-simple-v1` estimator, quality veto applied):

| Scenario         | Control tokens | Treatment       | Reduction                      |
| ---------------- | -------------- | --------------- | ------------------------------ |
| SCN-01           | 475            | 174 (gctx-only) | ~63%                           |
| SCN-02           | 574            | 215             | ~63%                           |
| SCN-04           | 494            | 170             | ~66%                           |
| SCN-10 (ceiling) | 400            | 335             | ~16%                           |
| SCN-30 (guard)   | 171            | 166 + block     | safety win, not token headline |
| SCN-32 (tax)     | 184            | 170             | ~8% validation tax             |

Edit ceilings SCN-11/12 may show **higher** treatment tokens when gctx payload
is added on top of required file reads — labelled `ceiling`, not `achieved`.

## Tier B

- Driver decision: **custom MCP host** via `ANVIL_DEVACC_DRIVER=external`
  writing `external-results.json`.
- Built-in `dry-run` scaffolds schema-valid B records from Tier A; notes mark
  them non-publishable.
- SCN-40 dry-run is a composite scaffold only.

## Claims

See `benchmarks/devacc/claims-policy.md`. No public marketing numbers are
authorised from this dry-run window.
