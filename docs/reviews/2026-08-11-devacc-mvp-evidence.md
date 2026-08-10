# DEVACC Tier A MVP evidence note

| Type  | Authority | Owner  | Status | Freshness  |
| ----- | --------- | ------ | ------ | ---------- |
| Guide | Advisory  | DEVACC | Live   | 2026-08-11 |

| Upstream                                                                                                    | Downstream                                                 |
| ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| [`dev-acceleration-benchmark-spec.md`](../architecture/dev-acceleration-benchmark-spec.md), DEVACC-001..006 | `plans/modules/dev-acceleration-benchmarks.aps.md`, claims |

## Scope

Tier A only (DEVACC-001..006). Tier B live agent evidence is out of this note.

| Field                  | Value                                            |
| ---------------------- | ------------------------------------------------ |
| Branch                 | `feat/devacc-complete`                           |
| Suite                  | Tier A deterministic scripts                     |
| Publishable hero claim | **No** — task-level marketing claims need Tier B |

## Commands

```bash
cargo test -p anvil-bench --lib -- devacc_
cargo run -p anvil-bench --bin anvil-bench -- devacc --tier A
```

## Sample paired reductions (`gctx-simple-v1`, quality veto)

| Scenario         | Control tokens | Treatment       | Reduction                      |
| ---------------- | -------------- | --------------- | ------------------------------ |
| SCN-01           | 475            | 174 (gctx-only) | ~63%                           |
| SCN-02           | 574            | 215             | ~63%                           |
| SCN-04           | 494            | 170             | ~66%                           |
| SCN-10 (ceiling) | 400            | 335             | ~16%                           |
| SCN-30 (guard)   | 171            | 166 + block     | safety win, not token headline |
| SCN-32 (tax)     | 184            | 170             | ~8% validation tax             |

Edit ceilings SCN-11/12 may show higher treatment tokens when a gctx payload is
added on top of required file reads — labelled `ceiling`, not `achieved`.

## Docs closeout

| Field               | Value                                                     |
| ------------------- | --------------------------------------------------------- |
| **Type**            | Review (internal measurement evidence)                    |
| **Does not change** | Product runtime; public marketing numbers                 |
| **Next**            | DEVACC-007..010 remain for Tier B / claims when scheduled |
