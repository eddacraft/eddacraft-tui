# Anchor Re-Scoring Snapshot — 2026-06-03

**Triggered by:** trigger 1 (LANGTS hit 6/6 and first T3 pass complete via PR #2125; NBI RSTLAN re-eval flagged at LANGTS gate) **Session owner:** @aneki (planning agent under dev-workflow for NBI completion)
**Surveyed user mix at time of snapshot:** Anvil (primary, Rust-heavy substrate + daemon + CLI), User B (confirmed prior), User C (Python + TS)

## Candidates

| Candidate                   | Demand | Blast | Strategic | Pack unlock | Composite | Δ from prior |
| --------------------------- | ------ | ----- | --------- | ----------- | --------- | ------------ |
| TypeScript                  | high (Anvil + packs + User B/C) | high (framework surface) | high (packs + early T3 proof) | high (5 Track 4 packs gated) | high | baseline (LANGTS complete) |
| Rust                        | high (Anvil dogfood: kernel, daemon, intercept, CLI; User B systems) | high (systems code, unsafe, crates, workspace mods) | critical (self-governance credibility for "governs systems code"; Rust catching up faster than 2026-04-26 assumption) | high (unlocks pack-tokio + future systems packs; enables Rust T3 for Anvil's own substrate before wider adoption) | high (elevated) | ↑ from prior (LANGTS gate noted "Rust catching up faster"; post v0.7.x Rust migration + daemon working makes self-audit urgent) |
| Python                      | medium (User C) | medium | medium | medium (Python-substrate LLM + Django/FastAPI) | medium | stable |

## Outcome

- [x] Sequence unchanged — anchor work proceeds. TS (complete) → Rust (now scoping to Ready) → Python.
- [ ] Sequence changed within Track 1 — pause; record amendment in spec §17.
- [ ] Tail-wave membership changed — update LANGTAIL.
- [ ] Promotion candidacy surfaced — open ADR.

## Notes

- The 2026-04-26 LANGTS re-scoring gate (solo self-review recorded in lang-ts-audit.aps.md) already flagged the dogfood signal for Rust. LANGTS-006 + PR #2125 close + CIB-031 dependency scoping (PR #2128) + kernel prereqs (LANGTS-005 Merged #2096) now unblock concrete RSTLAN work items.
- Rust demand elevated by: Anvil's own implementation now primarily Rust (post ADR-012/033/040/061 etc), daemon is the protection surface, intercept/witness/kernel all Rust. Governing the substrate the product ships in is table-stakes for trust.
- Blast radius for Rust T3 higher than initial model because of crate boundaries, mod/use re-exports, workspace layouts, unsafe/FFI, Serde hygiene — all directly relevant to Anvil's own code and early users doing systems work.
- Strategic: without Rust T3, Anvil cannot dogfood its own "architecture-validate" and layer claims on its primary language; this weakens the adoption narrative for Rust-using teams.
- Pack unlock: pack-tokio (and later Axum etc) explicitly gated on Rust substrate T3 (per language design §8.1 and ADR-027).
- No new surveyed users with Go-heavy or other stacks that would reorder Track 1; Python remains #3.
- Re-scoring owner gap (no permanent) noted; this invocation uses the NBI owner (@aneki) + separate review expectation per anti-patterns. Future invocations should name a standing owner per §17.3.
- Snapshot stored to support LANGTAIL-001 grammar maturity audit and future anchor gates.

References:
- lang-ts-audit.aps.md (gate run + 6/6 complete)
- plans/index.aps.md (NBI RSTLAN re-eval row)
- docs/guides/anchor-rescoring-process.md
- 2026-04-08-language-and-coverage-design.md §6, §8.1, §16.5 #8
