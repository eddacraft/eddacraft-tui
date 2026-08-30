# Change Evidence Graph

| ID | Owner | Status |
| --- | --- | --- |
| CEG | @aneki | Proposed |

**Last reviewed:** 2026-08-30 — module reconciled after the scoped Council
repair re-review `council-599eaa64` passed all C-001..013 findings and the
operator accepted ADR-135. CEG-001..006 remain Proposed; no
CEG/CAG/trusted-receipt implementation exists.

## Purpose

Provide an exact, bounded, fail-honest revision-delta projection that lets
conformance consumers test deterministic graph-semantic declarations against
committed Git and Graph v2 evidence.

The first public-facing use case is Verified Change Declarations under the
`intent-claim-integrity` tag. CEG is planless and independent of APS at
runtime.

## In Scope

- exact base/head committed-object selection and snapshot vector;
- public-symbol-surface, dependency-shape and privilege-surface predicates;
- predicate-specific completeness manifest and cold-build oracle;
- background-only, memory-only, single-flight execution;
- deterministic bounded identity-only projection;
- correctness, privacy, resource, contention, recovery and DEVACC graduation
  evidence;
- a later CONF consumer after CEG graduates.

## Out of Scope

- general graph/document search or intent inference;
- behavioural or API equivalence;
- working-tree evidence;
- a resident or persistent CEG;
- CAG implementation, build execution or platform inference;
- a new Test Evidence graph/API or raw CI-log retention;
- supply-chain/SBOM ownership;
- default-on or token-benefit claims before graduation evidence.

## Interfaces

**Depends on:**

- ADR-134 and CONF canonical binding/outcome semantics;
- canonical bounded replacement-disabled Git extraction;
- GV2 stable identities and owning semantic/dependency/trust projections;
- GCTX sealed DTO and egress bounds;
- GBASE committed-object and single-flight precedents;
- ADR-031, RLB and DEVACC evidence authorities.

**Exposes:**

- `CegSnapshotVector`;
- predicate-specific CEG outcomes and reason codes;
- bounded witnesses and completeness summaries;
- a future CONF-010 evidence seam.

## Hold condition

Do not promote implementation until:

- [x] ADR-135 is Accepted after the single scoped Council re-review;
- [ ] CEG-001 freezes exact supported-language semantics and the cold oracle;
- [ ] no existing graph/API can provide the same complete revision-pair answer;
- [ ] all persistence remains explicitly out of the first implementation;
- [ ] numeric bounds and existing resource/hot-path ceilings are testable;
- [ ] the owning documentation and diagram impact are approved.

## Work Items

### CEG-001: Contract, predicate and oracle spike

- **Status:** Proposed
- **Intent:** Freeze the CEG snapshot vector, authority references, reason codes,
  three initial predicates and the exact cold-build oracle.
- **Expected Outcome:** Versioned kernel contract and fixtures prove supported
  cold base/head results, every completeness disposition and honest unsupported
  outcomes without adding a resident graph.
- **Files:** `crates/anvil-kernel-types/`, a bounded spike under
  `crates/spike/`, and CEG component documentation chosen during plan-ready.
- **Validation:** focused kernel/spike tests plus fixture serialisation goldens.
- **Dependencies:** ADR-135 Accepted, CONF-004
- **Confidence:** medium

### CEG-002: Exact committed base/head projection

- **Status:** Proposed
- **Intent:** Build predicate inputs for two exact committed revisions using Git
  objects and supported Graph v2 extractors.
- **Expected Outcome:** Complete path/blob manifest; cold-build parity; compatible
  shared-base reuse parses exactly changed supported blobs; mismatch,
  unsupported and budget outcomes fail honest.
- **Validation:** parity, unsupported-language, parse-failure, schema/generation,
  changed-policy and 1/20/200/1,000-path fixtures.
- **Dependencies:** CEG-001, GBASE/GV2 authorities
- **Confidence:** low

### CEG-003: Bounded lifecycle and concurrency

- **Status:** Proposed
- **Intent:** Add cross-process per-key single-flight, global producer quota,
  coalescing, cancellation, priority and atomic in-memory generation.
- **Expected Outcome:** One producer per key, at most two per host, zero partial
  publication, descendant cleanup, four queued entries per repository/16 per
  host, two live cursor sessions per principal/eight per workspace/32 per host,
  60-second idle and five-minute absolute TTLs, stable broker-enforced
  principal plus workspace/host rolling credits, 256 MiB repository/384 MiB
  host retained-state high-water and no persistent bytes.
- **Validation:** herd, ref-churn, cancellation, queue-overflow, cursor-chain,
  session-restart, sequential fresh-CLI credit, TTL, high-water release,
  corruption, recovery and contention tests.
- **Dependencies:** CEG-002
- **Confidence:** low

### CEG-004: Identity-only projector

- **Status:** Proposed
- **Intent:** Project CEG outcomes through the existing authorised GCTX choke
  point without exposing graph internals or source.
- **Expected Outcome:** Maximum 200 items/64 KiB per page and 1 MiB per session;
  snapshot-bound cursors, deterministic pagination, omitted counts, narrowing
  guidance and structural no-leak tests.
- **Validation:** DTO goldens, adversarial fan-out and privacy/no-leak tests.
- **Dependencies:** CEG-002, CEG-003
- **Confidence:** medium

### CEG-005: Performance and operations graduation

- **Status:** Proposed
- **Intent:** Prove that CEG preserves hot-path/resource budgets and behaves
  honestly at representative and stress scale.
- **Expected Outcome:** Zero hot-path reachability; the low-priority
  `ceg-background-v1` process tree remains within 100% CPU/256 MiB per producer
  and 200% CPU/384 MiB per host; the extended watch/intercept/MCP/CEG resource
  harness retains 800% CPU/700 MiB RSS and every ADR-031 ceiling with at most 5%
  paired warm-p95 regression. The separate CEG, CAG, Test Evidence and combined
  matrix is recorded across repository tiers 1k/5k/10k/25k/50k/100k and delta
  tiers 1/20/200/1,000 before any default-on proposal.
- **Validation:** graph hot-read, IPC round-trip, extended process-tree
  resource-budget, paired contention, scale, churn and recovery benches under
  ADR-135's pinned runner and sample protocol.
- **Dependencies:** CEG-003, CEG-004
- **Confidence:** low

### CEG-006: Non-APS value trial and disposition

- **Status:** Proposed
- **Intent:** Measure whether CEG materially improves external coding and review
  tasks rather than anvil's own dev loop.
- **Expected Outcome:** Exact Tier A goldens and at least 10 paired successful
  Tier B tasks across control/CEG/degraded arms, with no quality veto, no rubric
  regression, non-negative success delta and at least 20% median total-token
  reduction before a material-benefit/default-on claim. Otherwise narrow or
  park the surface.
- **Validation:** DEVACC non-APS scenarios and reviewed evidence report.
- **Dependencies:** CEG-005
- **Confidence:** low

## Follow-on boundaries

- **CONF-010** will consume only graduated, predicate-specific CEG evidence.
  It will not reinterpret `no-behaviour-change` or `refactor-only` as proven.
- **CAG** requires its own explicit authority schema and admission decision. Its
  first facts are declared build/platform relationships, not inferred support.
- **Test Evidence** extends `anvil_affected_tests` with typed trusted-receipt
  origins after producer/trust admission; no competing API is permitted.
