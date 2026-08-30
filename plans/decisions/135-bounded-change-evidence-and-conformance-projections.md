# ADR-135: Bounded change-evidence and conformance projections

## Status

Accepted 2026-08-30 (operator; scoped Council repair re-review
`council-599eaa64`)

## Date

2026-08-30

## Context

ADR-134 admits deterministic intent conformance when a claim is checked against
complete, claim-appropriate evidence. Git already proves closed path and
file-class predicates. It cannot prove graph-semantic predicates such as whether
a dependency shape or public symbol set changed.

The tempting answer is to add several resident graphs. That would duplicate
Graph v2 authorities, expand persistence and privacy risk, and make a token
saving claim without first proving product value or resource cost. The design
instead needs one exact change-evidence surface and narrowly joined projections
whose positive answers remain trustworthy when evidence is incomplete.

The first non-APS use case is **Intent & Claim Integrity**. A pull request may
publish a machine-readable Verified Change Declaration. anvil deterministically
extracts the declaration, binds it to immutable source identity, and reports
whether the exact committed change supports it. The machine tag is
`intent-claim-integrity`.

## Decision

### 1. Admit one graph and two bounded follow-on projections

The first new surface is the **Change Evidence Graph (CEG)**: an ephemeral,
revision-delta projection over existing Git, Graph v2 semantic/dependency, and
trust authorities. CEG is not a new source of truth, does not copy foreign nodes,
and is not a repository-wide mega-graph.

Two later surfaces are projections, not separately resident graphs:

- the **Contract & Authority Graph (CAG)** joins explicit, versioned repository
  declarations to existing graph identities for deterministic conformance;
  initial build and platform declarations belong here;
- **Test Evidence** extends the existing `anvil_affected_tests` result with
  sparse, typed receipt references from trusted CI. It does not introduce a
  competing affected-tests API or retain raw logs.

CEG lands and is measured first. CAG and Test Evidence remain default-off until
their own deterministic predicate, privacy tests, and graduation evidence pass.

### 2. Product predicates are primary; token saving is secondary

A surface is admitted only when it answers a deterministic prevention,
conformance, trust, or provenance predicate. Initial predicates are:

| Surface | Admitted predicate | Exact meaning |
| --- | --- | --- |
| CEG | `public-symbol-set-unchanged` | The ordered set of supported public symbol identities is identical at the bound base and head. It does not prove API or behavioural equivalence. |
| CEG | `dependency-shape-unchanged` | The ordered supported dependency-edge identities are identical at the bound base and head. |
| CEG | `privilege-surface-unchanged` | The ordered supported trust/boundary edge identities are identical at the bound base and head. |
| CAG | `declared-contract-chain-complete` | Every explicitly declared required contract edge resolves under the independently anchored authority revision. |
| CAG | `declared-platform-evidence-complete` | Every explicitly declared target/platform/toolchain tuple has the required compatible trusted receipt. |
| Test Evidence | `required-test-receipts-complete` | Every explicitly required test target has a compatible, completed trusted receipt for the exact subject and configuration. |

Observed absence of a test, edge, or failure never proves that a change is
unaffected. `no-behaviour-change` and `refactor-only` remain
`not-evaluated` until a future predicate can prove their exact semantics.
Token and context reduction are measured benefits, never authority for a
verdict.

### 3. Exact ownership and snapshot vector

Each datum retains its owning authority:

| Datum | Authority |
| --- | --- |
| Repository, commits, trees, blobs, statuses and paths | replacement-disabled canonical Git extraction from ADR-134 |
| Symbols, dependencies and supported trust edges | the owning Graph v2 projection and schema |
| Contract/build/platform declarations | an explicit versioned repository contract loaded from an independent or base-tree anchor |
| CI execution facts | approved external producer and immutable receipt |
| Declared intent | canonical conformance contract with source reference and digest |

A joined query is valid only inside one exact snapshot vector containing:
repository identity; base/head commit and tree object IDs; every consulted path
and blob ID; an ordered projection-binding vector keyed by owning authority and
base/head side; contract source revision and digest; manifest, lockfile,
toolchain, workflow and configuration digests when relevant; and CI subject,
producer, run and receipt identities. Each projection binding carries the
authority kind, side, revision and tree object IDs, coverage digest, schema
version, generation, parser version and tool version. One tuple cannot stand
for multiple owning projections or both sides. A missing, duplicate or
mismatched binding is reason-coded `not-evaluated`.

Joins use exact stable identities. A projection may retain an authority kind,
identity and digest reference, but never copy a foreign graph node or silently
coerce an incompatible generation.

### 4. No self-authorisation and no forged CI evidence

A declaration changed by the evaluated range is evidence, not authority for the
same range. A positive CAG result therefore uses the base revision or a
separately trusted authority root. A changed mapping, contract, workflow or
required-gate declaration is reported as `policy-change` and cannot authorise
itself.

A CI receipt is trusted only when its versioned trust envelope is authenticated
against an independently configured producer policy. The envelope binds the
policy revision and digest; issuer or workload identity; key or trust-root
identity; signature or transparency-entry digest; exact commit and tree;
workflow path, workflow blob and action digests; event and protected ref;
check/run identity; producer; runner class and identity; anvil binary;
instrumentation; configuration; platform; toolchain and lockfile digests;
target; schema; issued, completed and expiry times; and final conclusion. The
verifier records the authentication mechanism, trust-policy digest and
verification time. Missing, invalid, revoked, expired or mismatched fields make
the receipt untrusted. User prose, mutable check names, status badges and
unbound logs are not receipts.

### 5. Predicate-specific completeness and monotonic outcomes

Only complete, identity-matched, trusted, supported and unexpired evidence may
produce a positive or unchanged result. Unsupported language, parser failure,
stale or mismatched identity, untrusted producer, truncation, ambiguity,
disabled surface, budget exhaustion or incomplete receipt yields a stable
reason-coded `not-evaluated`.

A proven violation is monotonic and may survive later incompleteness. Partial
evidence may never prove absence or become a cached negative. Every supported
member receives a disposition, and parity fixtures compare supported results
with a cold-build oracle exactly.

### 6. Bounded execution and lifecycle

CEG evaluates exact committed Git objects only; it never reads a working tree.
The canonical replacement-disabled extraction and process-tree timeout contract
from ADR-134 applies. Base and head Graph v2 projections are content-addressed
by the snapshot vector.

The first implementation is explicit, background-only and memory-only:

- no construction, traversal, receipt ingestion or I/O is reachable from
  `midEdit`, `preWrite`, `save`, gate-enforcement, or daemon certification
  hot paths;
- at most one producer runs per repository/snapshot key and at most two CEG
  producers run per host; duplicate requests coalesce;
- the priority queue holds at most four entries per repository and 16 per host;
  overload returns `surface.busy`, and cancellation releases its reservation
  within one second;
- every producer runs in the low-priority `ceg-background-v1` resource class
  with a complete owned process tree, at most 100% CPU and 256 MiB peak RSS;
  the host-wide CEG aggregate is at most 200% CPU and 384 MiB peak RSS;
- a producer or aggregate resource breach terminates the owned process group,
  publishes no generation and returns reason-coded `not-evaluated`;
- newer requests cancel obsolete queued work; cancellation kills descendant
  processes and publishes no partial generation;
- queues are bounded, interactive work has priority, and overload returns a
  typed unavailable/budget outcome;
- a generation is published atomically only after complete validation;
- live retained state is capped at 256 MiB per repository and 384 MiB per host;
  memory and reservations are released within one second after the last
  request, session close, cancellation or TTL expiry.
  There is no persistent CEG/CAG/Test Evidence artefact, retention store or
  historical receipt cache in this decision.

Persistence requires a separate graph-specific ADR covering format, owner-only
storage, integrity, retention, refcounts, TTL/high-water garbage collection,
corruption recovery, privacy and migration/discard rules.

### 7. Fixed projection and egress bounds

The first implementation reuses shipped GCTX bounds:

| Bound | Limit |
| --- | ---: |
| Changed paths accepted from a caller | 200 |
| Reverse traversal depth | 2 |
| Result items per page | 200 |
| Dependency/test intermediate walk | 10,000 nodes |
| Affected symbols materialised | 20,000 |
| Edge identities enumerated | 50,000 |
| Serialized identity-only response page | 64 KiB |
| Cursor | 16 KiB |
| One cursor session total egress | 1 MiB |
| Concurrent cursor sessions | 2 per principal; 8 per workspace; 32 per host |
| Cursor idle/absolute TTL | 60 seconds / 5 minutes |
| Session creation | 10 per principal per rolling minute |
| Rolling egress credit | 4 MiB per principal per rolling minute |
| Rolling traversal credit | 100,000 identities per principal per rolling minute |
| Workspace rolling credit | 16 MiB and 400,000 identities per rolling minute |
| Host rolling credit | 64 MiB and 1,600,000 identities per rolling minute |

A change over 200 paths must be internally chunked while preserving complete
coverage or return `not-evaluated:budget.changed-paths`; the first 200 paths
are never treated as the whole change. Ranking, grouping, pagination and
truncation are deterministic. Every bounded response reports omitted counts and
narrowing guidance. Cursors bind the whole snapshot vector and query
fingerprint.

A principal is one authenticated MCP connection. For CLI/check clients it is a
stable daemon-enforced identity derived from the authenticated operating-system
user and canonical repository identity, retained across client-process
lifetimes; the raw user identity never enters a response. All projection work
and credit accounting occurs in the long-lived local broker. A CLI/check client
without that broker returns `surface.unavailable` and cannot run an unmetered
local producer. Exhausted principal, workspace or host session, egress or
traversal credit returns `surface.busy` or the relevant budget outcome without
starting fresh traversal. Fixtures restart sessions, chain cursors and launch
sequential fresh CLI processes to prove that none resets rolling credits.

### 8. Privacy and egress

Every surface is identity-only by default. Allowed fields are hashes, relative
identities, source references and digests, reason codes, bounded counts, sparse
target-level aggregates and bounded receipt references. Raw PR bodies, commit
messages beyond the already normalised claim, source text, logs, comments,
environment data, absolute paths, PII, secrets and line-level coverage history
are forbidden.

Each sealed DTO receives structural no-leak tests before any agent egress.
Test Evidence projects only the latest compatible receipt per
`{target, platform, toolchain}`; historical receipts remain in their external
authority.

### 9. Performance and value graduation

The following gates are declared before implementation:

1. **Hot-path invariant:** zero invocation from interactive and enforcement hot
   paths. Existing ADR-031 warm p95 ceilings remain unchanged: 50/80 ms
   service/round-trip for buffer/pre-write and 80/120 ms for save-time.
2. **Resource invariant:** the `ceg-background-v1` process tree obeys the
   section 6 per-producer and host aggregate limits. The concurrent resource
   harness samples that tree alongside watch, intercept and MCP while retaining
   the existing all-process 800% CPU/700 MiB RSS ceiling, every ADR-031 absolute
   latency ceiling and at most 5% paired warm-p95 regression.
3. **Deterministic work gate:** when a compatible shared base exists, head
   construction parses exactly the changed supported blobs, not the base again.
   Fixtures cover repository tiers 1k, 5k, 10k, 25k, 50k and 100k files and
   delta tiers 1, 20, 200 and 1,000 paths. The 1,000-path tier must either chunk
   completely or fail honest.
4. **Scale/resource gate:** the existing graph-memory ceiling of 500 MiB at the
   2,000-file/~100k-LOC reference fixture remains binding and growth must be
   linear or sub-linear. Memory-only CEG adds zero persistent disk bytes.
5. **Query/egress gate:** all bounds in section 7 are asserted at representative
   and adversarial fan-out. CAG exercises 1, 200 and 10,000 consumers; Test
   Evidence exercises 1, 200 and 10,000 candidate tests and 1, 10 and 50 receipt
   histories.
6. **DEVACC value gate:** Tier A payload goldens are exact. A default-on or
   material-token-benefit claim additionally requires at least 10 paired
   successful non-APS, non-internal-dev-loop tasks with model and anvil SHA
   pinned, no quality veto, no rubric regression, success-rate delta at least
   zero, and median total-token reduction of at least 20% for the relevant
   projection.

Prototype stop gates are numeric even where a quiet-runner baseline does not yet
exist:

| Axis | CEG | CAG | Test Evidence | Combined |
| --- | ---: | ---: | ---: | ---: |
| Background CPU | ≤100% per producer; ≤200% host | ≤50% | ≤25% | ≤200%; all-process ≤800% |
| Peak RSS | ≤256 MiB per producer; ≤384 MiB host | ≤64 MiB | ≤64 MiB | ≤384 MiB; all-process ≤700 MiB |
| Cold build | p95 ≤30 s/5k, ≤120 s/20k, ≤300 s/100k files | p95 ≤30 s/10k consumers | p95 ≤5 s/50 receipts | p95 ≤300 s |
| Warm/incremental | p95 ≤30 s/200 changed blobs | p95 ≤1 s/200 authority facts | p95 ≤100 ms/receipt and ≤5 s/50 | p95 ≤30 s |
| Ingest | p95 ≤30 s/200-path manifest | p95 ≤1 s/200 authority facts | p95 ≤100 ms/receipt and ≤5 s/50 | p95 ≤30 s |
| Invalidation | ≤1 s | ≤1 s | ≤1 s | ≤1 s and no stale positive |
| Query | p95 ≤80 ms; p99 ≤250 ms | p95 ≤80 ms; p99 ≤250 ms | p95 ≤80 ms; p99 ≤250 ms | p95 ≤120 ms; p99 ≤500 ms |
| Interactive contention | ADR-031 absolute ceilings and ≤5% paired warm-p95 regression | same | same | same |
| Recovery | ≤30 s to recomputed or not-evaluated state | ≤30 s | ≤30 s | ≤30 s |
| Incremental memory at 10k | ≤32 MiB | ≤32 MiB | ≤32 MiB | ≤64 MiB within absolute ceilings |
| Persistent disk | 0 bytes | 0 bytes | 0 bytes | 0 bytes |
| Egress | section 7 page/session and rolling-credit bounds | same | same | same |

The cold figures are conservative fail-safe ceilings, not a claim of expected
speed. Calibration runs on a dedicated Linux x86-64 runner with four dedicated
cores, 8 GiB RAM, local NVMe, release binaries, no network access and a recorded
image digest. It uses 20 independent cold/ingest/recovery samples, 1,000 warm
queries and nearest-rank percentiles. The contention comparison is paired on
the same repository SHA and fixture with projections disabled and enabled.
Every table cell is a stop gate; a runtime breach cancels or kills the owned
process group and returns `not-evaluated`. Calibration may tighten these gates
through an ADR amendment; it may not silently loosen them. Receipt
expiry/retention and memory-session high-water release are tested even though
persistence and historical receipt storage are absent.

ADR-031 continues to classify background work as report-only; this ADR does not
smuggle background timings into its interactive rubric. No projection becomes
resident or default-on while a required axis is unmeasured. DEVACC records
tokens, context bytes, latency and quality separately for CEG, CAG, Test
Evidence and the combined arm so an aggregate win cannot hide a costly surface.

## Consequences

- Intent & Claim Integrity gains exact graph evidence without turning anvil into
  a planning or retrieval system.
- CEG can prototype with bounded, disposable state and fail-honest results.
- CAG and Test Evidence have explicit admission gates rather than implied
  permission to become resident graphs.
- Large, unsupported or mismatched changes may remain `not-evaluated`; this is
  preferable to a false clean result.
- The 20% token threshold is a product graduation threshold, not an assertion
  that current evidence has already met it.

## References

- [ADR-031](031-validation-latency-rubric.md)
- [ADR-063](063-gv2-hot-path-boundary.md)
- [ADR-069](069-graph-v2-persistence.md)
- [ADR-105](105-shared-base-graph-persistence.md)
- [ADR-134](134-intent-conformance-gating.md)
- [Change Evidence Graph specification](../specs/2026-08-30-change-evidence-graph.md)
- [Graph v2 foundation](../../docs/architecture/graph-v2-foundation-spec.md)
- [Graph Context Delivery](../../docs/architecture/graph-context-delivery-spec.md)
- [Resource budgets](../../docs/policies/resource-budget.md)
- [Dev Acceleration benchmark](../../docs/architecture/dev-acceleration-benchmark-spec.md)
