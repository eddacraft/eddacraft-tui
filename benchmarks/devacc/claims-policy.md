# DEVACC claims packaging

External or marketing numbers may only cite evidence produced by this suite (or
GCTX-031 for the narrower payload micro claim). Every public number must include
**scenario id + arm + tier + date + model (Tier B) / Anvil SHA**.

## Allowed claim → evidence map

| Claim type                                                          | Allowed evidence                                         |
| ------------------------------------------------------------------- | -------------------------------------------------------- |
| Identity graph payloads are ~X% smaller than reading impacted files | GCTX-031 only (payload micro)                            |
| Agents complete impact questions with ~X% fewer tokens              | Tier A SCN-01/02 **or** Tier B with n≥10                 |
| Developer acceleration reduces tokens on real feature work by ~X%   | Tier B SCN-40 (or composite SCN-10–14) with quality veto |
| Anvil prevents secret/boundary footguns in the agent loop           | SCN-30/31 true-positive rate, not token %                |
| Engine latency / RSS                                                | Kernel + RLB benches — not this suite                    |

## Quality veto

Failed tasks must not be averaged into token-win headlines. Report success rate
alongside any token reduction. Guard scenarios may show a **token tax**; report
safety wins separately.

## Caveats (attach or hash)

1. Estimator ≠ billing tokens (Tier A uses `gctx-simple-v1`).
2. Identity-only default; `egress=on` runs are not mixed into identity means.
3. Competent control required (search + selective reads, not whole-repo
   strawman).
4. Success-conditioned means only with explicit success rate.
5. Safety tax is a feature, not a regression, for guard arms.
6. Model drift invalidates Tier B history until re-baselined.
7. `full-accel` without skill enforcement is labelled `tools-available`.
8. Cold/unready graph runs are invalid for token claims.

## SCN-40 hero rule

SCN-40 is the **only** scenario allowed in external "developer acceleration"
hero claims unless a narrower claim explicitly names its scenario id. SCN-40
requires secondary metrics and success rate — tokens alone are not enough.
