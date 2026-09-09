# DEVACC MVP evidence note (Tier A + dry-run B)

| Type  | Authority | Owner  | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----- | --------- | ------ | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | DEVACC | Live   | Last reviewed 2026-09-09 GATT-001 amended the GCTX delivery spec with ADR-142 self-attestation (shared Attestation block). This guide's consumer-routing prose is unchanged; teaching the new disclosures is GATT-006. No depicted topology, trust boundary, or workflow moved. Prior review 2026-09-03 against the CIB-398 `docs/architecture/dev-acceleration-benchmark-spec.md` re-date — the graph tools now refuse a nested `workspaceRoot`; the recorded runs used the checkout root, so this evidence note is unaffected. Prior review 2026-09-01 against the pack-12 re-date (pointer freshness only). Prior review 2026-08-30 against the same spec for the same pointer-only freshness. Prior review 2026-08-16 for #3915 (metadata-only re-date). Same finding on 2026-08-13 for DOCFRESH-005 |

| Upstream                                                                                                                                                                               | Downstream                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| [`dev-acceleration-benchmark-spec.md`](../architecture/dev-acceleration-benchmark-spec.md), DEVACC-001..010 (no live agent yet) `docs/architecture/dev-acceleration-benchmark-spec.md` | `plans/archive/modules/dev-acceleration-benchmarks.aps.md` |

## What this note covers (no headless-agent choice required)

| Surface                                   | Status        | Notes                                                                   |
| ----------------------------------------- | ------------- | ----------------------------------------------------------------------- |
| Tier A deterministic suite                | Done          | navigate / edit ceiling / plan / guard scripts                          |
| Tier B dry-run + external driver contract | Done          | `ANVIL_DEVACC_DRIVER=dry-run\|external`; not Claude/Cursor-specific     |
| Claims packaging                          | Done          | `benchmarks/devacc/claims-policy.md`                                    |
| Live pinned-model agent runs (n≥10)       | **Not done**  | Needs credentials + external driver implementation; not publishable yet |
| Opt-in nightly / PR gate                  | Proposed only | Off by default; not required for Complete                               |

## Commands

```bash
cargo test -p anvil-bench --lib -- devacc_
cargo run -p anvil-bench --bin anvil-bench -- devacc --tier A
cargo run -p anvil-bench --bin anvil-bench -- devacc --tier B --dry-run
```

## Sample Tier A paired reductions (`gctx-simple-v1`, quality veto)

| Scenario         | Control tokens | Treatment       | Reduction                      |
| ---------------- | -------------- | --------------- | ------------------------------ |
| SCN-01           | 475            | 174 (gctx-only) | ~63%                           |
| SCN-02           | 574            | 215             | ~63%                           |
| SCN-04           | 494            | 170             | ~66%                           |
| SCN-10 (ceiling) | 400            | 335             | ~16%                           |
| SCN-20 (plan)    | 276            | 229             | ~17%                           |
| SCN-30 (guard)   | 171            | 166 + block     | safety win, not token headline |
| SCN-32 (tax)     | 184            | 170             | ~8% validation tax             |

## Tier B dry-run

Dry-run records reuse Tier A scripts (and a composite SCN-40 scaffold) with
`tier: B` and `model: dry-run`. Notes mark them **non-publishable**. They prove
report schema + runner wiring only.

## Docs closeout

| Field               | Value                                                              |
| ------------------- | ------------------------------------------------------------------ |
| **Type**            | Guide (internal measurement evidence)                              |
| **Does not change** | Product runtime; public marketing numbers                          |
| **Next**            | Live external driver runs when credentials exist; keep 011/012 off |
