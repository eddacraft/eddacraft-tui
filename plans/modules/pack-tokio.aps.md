<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Tokio Semantic Pack (Track 4)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| PACKTOK | —     | Draft  |

**Last reviewed:** 2026-04-26

> Note (2026-04-26): "Rust substrate" = Rust code being analysed. The pack
> itself ships as a Rust crate `crates/anvil-pack-tokio/` per
> [ADR-027](../decisions/027-pack-architecture.md). Anvil's own kernel
> (`crates/anvil-kernel/`, Tokio-based) is the dogfood target.

## Purpose

Per [2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§8.4 row 6. Catches Tokio async-runtime anti-patterns layered on Rust at
T2+. Demand: 2 (Anvil's own kernel uses Tokio + User B). Blast: high.
Strategic: supports.

Phase 2 deliverable (spec §9 step 14). Unblocks after `lang-rust` reaches
T2+ (i.e. anti-pattern catalogue + suppression — does not need full T3).

## In Scope

- Substrate language: Rust. Minimum substrate tier: **T2+** (relaxed from
  T3 — this pack only needs anti-pattern infrastructure, not full layer
  enforcement).
- Pack activation: detect `tokio` / `tokio::*` imports.
- Rule catalogue (per spec §8.4 row 6):
  - Blocking calls (`std::fs::*`, `std::io::*`, `std::sync::Mutex` lock)
    inside `async fn` / async blocks
  - `.await` while holding a `MutexGuard` / `RwLockGuard`
  - Unbounded channels (`tokio::sync::mpsc::unbounded_channel`)
  - `tokio::spawn` without retaining or explicitly detaching the
    `JoinHandle`
  - `select!` macros missing cancellation branches in long-lived loops

## Out of Scope

- Async-trait performance analysis.
- Runtime configuration policy (`#[tokio::main]` flavour, worker thread
  count).
- Tracing / tracing-subscriber integration policy.
- Other async runtimes (`async-std`, `smol`) — re-score on demand.

## Interfaces

**Depends on:**

- [`lang-rust`](./lang-rust.aps.md) — Rust at T2+ (does not require full
  T3).
- [`pack-pulumi`](./pack-pulumi.aps.md) — first consumer of the pack
  architecture; PACKPUL-001 lands the crate registry.
- [ADR-027](../decisions/027-pack-architecture.md) — pack architecture
  (symbol-graph access required for async-context reasoning, which depends
  on the kernel extractor refactor for Rust async items).

**Exposes:**

- Tokio rule catalogue. Anvil's own Rust kernel gets governed — dogfood
  case.

## Prerequisites

- `lang-rust` at T2+.
- [ADR-027](../decisions/027-pack-architecture.md) Accepted; PACKPUL crate
  skeleton landed.

## Ready Checklist

Change status to **Ready** when:

- [ ] RSTLAN at T2+.
- [ ] ADR-027 Accepted; PACKPUL-001 landed.
- [ ] Anvil kernel baselined.
- [ ] Owner named.

## Work Items

Anticipated:

- PACKTOK-001: `tokio` import detection.
- PACKTOK-002: Blocking-call-in-async rule.
- PACKTOK-003: `.await` while holding lock rule.
- PACKTOK-004: Unbounded-channel rule.
- PACKTOK-005: `JoinHandle` retention rule.
- PACKTOK-006: `select!` cancellation rule.
- PACKTOK-007: Validation against Anvil kernel + User B Rust code.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Blocking-in-async detection requires async-context awareness in the symbol graph | High | Pack architecture (PACKPUL ADR) must support symbol-graph access for Rust async items |
| `.await` while holding lock requires guard-lifetime tracking | High | Phase 1: heuristic on textual `.await` between `let _guard = …` and guard-drop; document as approximation |
| Unbounded-channel rule trips on legitimate test code | Medium | Test-file allowlist by default |
| Tokio-version churn breaks heuristics | Medium | Pin rules to a documented Tokio version range |

## Open Questions

- [ ] Async-context propagation in the symbol graph — Rust-side decision
      needed in PACKPUL-001 ADR?
- [ ] Should `tokio::sync::Mutex` (async-aware) be flagged differently
      from `std::sync::Mutex` in async contexts?
