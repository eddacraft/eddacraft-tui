# ADR-111: Graph-backed exact LSP references

## Status

**Proposed** — 2026-07-20. Synthesised by Planning Council
`plan-33b005f5`; the operator approved the council's scope and defaults.

## Date

2026-07-20

## Context

ADR-109 authorises a generic LSP surface for RTAI-005 but keeps that item
diagnostics-only. PR #3360 explored a broader LSP suite and exposed a category
error: graph dependants or caller declarations are not source reference
occurrences. Returning them from `textDocument/references` would look plausible
while producing incomplete or wrong editor navigation.

Exact references require a distinct architecture. The daemon needs a complete,
versioned occurrence model for a certified language tier; every location must
come from one immutable graph generation and bytes verified beneath one admitted
workspace root. The query must remain bounded under large or hostile workspaces,
must not steal capacity from save-time validation, and must cross the CE-5
egress boundary without source, absolute paths, hashes, or partial results.

The capability is useful only where it reinforces anvil's deterministic control
layer: it lets a person or agent inspect every governed use of a symbol before
changing it. General IDE productivity features remain outside this decision.

## Decision

### 1. Scope and ownership

- RTAI-005 owns production mid-edit diagnostics only.
- `anvil-graph-cache` owns occurrence structures, deltas, snapshot invariants,
  and persistence; Intercept owns anchored rebuild and atomic publication; GCTX
  owns anchored query projection, bounds/work credits, and CE-5/CE-11 semantics.
- LSPNAV owns the LSP projection and capability lifecycle.
- The first release implements exact `textDocument/references` only. Impact of
  change and affected-test discovery are later Proposed work with no v1 wire
  names or compatibility promise.
- PR #3360 is reduced to diagnostics-only. Navigation is rebuilt as fresh,
  main-based LSPNAV slices; unmerged experimental wires are not preserved.

### 2. Certified accuracy and capability advertisement

V1 ships one language tier chosen by evidence, not preference. A tier is
certified against a closed, versioned taxonomy of declarations and every
statically resolved use it claims to support. Unsupported or unresolved
constructs invalidate successful completeness; they never yield a partial
answer.

The server omits a global `referencesProvider` from `initialize`. After
`initialized`, it dynamically registers `textDocument/references` only for the
certified language, file scheme, and document selector, and only for launch
clients proven to support dynamic registration. Clients without that capability
are not LSPNAV-v1 clients. Out-of-contract requests fail with a stable
`TierNotCertified` reason; `null` is not used to disguise unsupported scope as a
complete empty result.

The certification identity covers the taxonomy, parser, extractor, resolver,
occurrence index, persistence schema, coordinate conversion, scheduler, and
protocol contract. A correctness-relevant change invalidates the affected tier
until its evidence digest is regenerated. A runtime invariant failure cancels
affected work, unregisters the tier for the session, and emits only bounded,
low-cardinality telemetry.

Registration state is driven by a daemon-owned, monotonic capability epoch over
an authenticated, bounded latest-value control subscription. The LSP process
receives an initial state and coalesced changes; it never infers capability from
the last query response. A control disconnect, daemon restart, disabled epoch,
or certification change cancels navigation and unregisters the provider. A
fresh enabled, certified state is required before re-registration on reconnect.

### 3. One immutable verified graph snapshot

`anvil-graph-cache` owns one immutable composite `GraphSnapshot`. It atomically
contains the symbol, dependency, and call graphs; occurrence index; indexed
content hashes; bounds state; canonical root identity; schema and certification
identities; and a monotonic generation. Readers pin one `Arc<GraphSnapshot>`;
partial graph and occurrence generations are never observable.

The intercept scan coordinator owns held-root anchors, parsing and rebuild
orchestration, cancellation, scheduling, and atomic publication. Graph-cache
locks are held only to pin or publish a snapshot; no filesystem I/O occurs under
those locks.

Persistence extends the existing sealed graph snapshot as an untrusted
acceleration artefact. Schema, integrity, root, bounds, certification, and
generation coupling are validated as one unit. Incompatible, corrupt, newer, or
downgrade-unsupported state is ignored and rebuilt; it is never migrated or
partially trusted. Until rebuild publishes a complete occurrence-capable
snapshot, references return `NotReady`.

For a query, the daemon pins the composite snapshot, derives a bounded
deduplicated result-file worklist, and verifies the initiating hash plus every
participating file through the same held `WorkspaceAnchor`. Exact coordinates
are derived from those verified bytes as explicitly tagged line/UTF-16 ranges.
The request also carries a bounded, authenticated, root-scoped open-document
manifest: one monotonic manifest generation plus workspace-relative path, LSP
document version, and content hash for every document open in that LSP session.
The manifest is a complete set for the session and contains no source. Any
participating open document absent from an internally inconsistent manifest, or
whose client hash differs from the indexed/verified hash, makes the whole result
`ContentModified`. A missing file, hash mismatch, unsafe
identity transition, cancellation, deadline, or final generation change returns
no locations.

### 4. Closed, bounded and private query contract

The protocol-neutral CE-5 outcome is closed and all-or-nothing:

- `Ready` — workspace-relative exact locations from one verified generation;
- `NotReady`, `ContentModified`, `Cancelled`, `Busy`, `DeadlineExceeded`,
  `Overflow`, `Disabled`, `InvalidQuery`, `Withheld`, or
  `ReferenceBudgetExhausted` — no locations.

`Withheld` is generic: a CE-3-sensitive, ignored, unsafe, cross-root, or
otherwise non-egressable participating file withholds the whole response and
does not reveal which file exists. Non-ready outcomes expose only stable enums
and safe retry guidance. Absolute paths, URIs, symbols, source, parser errors,
hashes, persistence names, precise credit balances, and partial results never
cross CE-5. LSPNAV only maps verified relative files and daemon-produced tagged
UTF-16 ranges to encoded file URIs and LSP DTOs; it does not read workspace
files or convert byte offsets.

Outcome selection follows one normative precedence at response sealing:
`Disabled` > `Cancelled` > `DeadlineExceeded` > `InvalidQuery` > `NotReady` >
`ReferenceBudgetExhausted` > `Busy` > `Overflow` > `Withheld` >
`ContentModified` > `Ready`. The gate, validation, admission, and execution
stages latch only privacy-safe facts; this order resolves simultaneous races
without leaking which later condition also occurred. LSP maps `Cancelled` to
`RequestCancelled`, `ContentModified` to `ContentModified`, `Busy` and
`DeadlineExceeded` to `ServerCancelled` with stable reasons, and all other
non-ready outcomes to `RequestFailed` with stable Anvil reason data.
Immediate fair-admission or queue-capacity refusal is `Busy`; an admitted
request that spends 100 ms in queue or reaches the one-second end-to-end bound
is `DeadlineExceeded`.

Queries use checked-in immutable safety ceilings that configuration may lower
but not raise. Successful-work defaults are calibrated from representative
Linux and Windows benchmarks. The v1 queue bound is 100 ms and the end-to-end
receipt-to-response deadline is one second. A 250 ms warm p95 is a release gate,
not an architectural promise; p99, maximum, memory, cancellation, soak, and
save-time contention are co-equal gates.

Navigation uses only a separate non-reserved scheduler class. Save-time
validation retains exclusive capacity that navigation cannot borrow, including
when idle. On a host without safe spare capacity, navigation degrades
explicitly rather than delaying save-time work.

Within the bounded navigation queue, admission is hierarchical and fair: at
most one running and one queued request per authenticated principal/canonical
root, at most one queued request per live session, and deterministic round-robin
selection across principals, then roots, then sessions. Reconnect cannot change
the principal/root parent identity or jump ahead of already admitted peers.

Repeated calls are bounded by daemon-owned hierarchical weighted work credits:

- a principal-wide aggregate derived from the authenticated OS peer/session
  lineage;
- a parent keyed by that principal plus the admitted anchor's canonical root;
- a fair child lane for each live LSP session.

Reconnect creates no fresh parent credit. Credit refills using monotonic time;
daemon restart is the explicit reset boundary. Expected work is reserved before
execution, actual work is charged, and only unused reservation is refunded.
Account state is globally bounded and inactive records expire deterministically.
Exhaustion returns `ReferenceBudgetExhausted` with coarse retry guidance and no
balance side channel.

### 5. Delivery and rollback

Delivery uses six independently gated, main-based stages:

1. make PR #3360 production diagnostics-only;
2. merge this ADR and the LSPNAV APS plan after reconciling them to that final
   diagnostics boundary;
3. land the closed taxonomy, certification, occurrence deltas, composite
   snapshot, and persistence with no query surface;
4. land the bounded anchored RPC as a separate trust boundary, disabled by
   CE-11;
5. land LSP projection and dynamic certified-tier registration;
6. run shadow, canary, soak, and release promotion as an evidence-producing
   stage.

CE-11 controls both direct location egress and capability registration through
the capability-epoch control subscription. Turning it off advances the epoch,
unregisters an active provider even while idle, cancels in-flight navigation,
and makes direct requests return `Disabled` without affecting diagnostics.
Older binaries ignore or reject newer cache state and rebuild, so rollback
requires no reverse migration.

## Rationale

Exact reference navigation cannot be projected from call/dependency edges.
Making occurrence identity part of the same immutable graph snapshot prevents
mixed-generation answers, while anchored verification closes stale-disk and
path-substitution races. Dynamic registration is narrower than static LSP
advertisement, but it is the only standard capability shape that can honestly
advertise one certified language tier. Separate scheduler capacity and
cumulative work credits protect anvil's primary save-time control path.

### Alternatives Considered

| Option | Pros | Cons |
| ------ | ---- | ---- |
| Certified occurrence snapshot plus dynamic LSP projection (chosen) | Exact, protocol-neutral, fail-closed, independently reviewable | Larger substrate; first release supports fewer languages and clients |
| Project references from caller/dependant declarations | Small diff over existing graphs | Semantically wrong and incomplete; cannot return occurrence ranges |
| Static `referencesProvider` with runtime null/errors | Supports more clients | Globally advertises a capability the server cannot honour for every document |
| Read files in the LSP process | Simpler daemon response | Duplicates trust logic and permits mixed roots/generations |
| Per-request caps only | Easy to implement | Reconnect and repeated-query abuse bypass the resource and disclosure boundary |
| Combine snapshot, RPC, and LSP in one PR | Fewer merges | Hides distinct persistence, egress, and protocol trust boundaries |

## Consequences

- **Positive:** exact and atomic references; one protocol-neutral source of
  truth; explicit language/client claims; bounded resource use; immediate
  rollback; save-time isolation.
- **Negative:** one certified tier at launch; clients without dynamic
  registration are excluded; verified cross-file reads add latency; persistence
  schema and certification evidence become coupled.
- **Risks:** incomplete taxonomy, stale or hostile workspace state, occurrence
  scraping, scheduler starvation, platform drift, downgrade incompatibility.
- **Mitigations:** differential and adjudicated golden corpora, whole-response
  verification, closed outcomes, hierarchical credits, exclusive save-time
  capacity, Unix/Windows evidence, shadow/canary rollout, and CE-11.

## References

- Planning Council: `plan-33b005f5`
- [ADR-109](./109-lsp-agent-integration-reconsidered.md)
- [ADR-069](./069-graph-v2-persistence.md)
- [ADR-084](./084-gctx-graph-handle-access.md)
- [ADR-031](./031-validation-latency-rubric.md)
- [RTAI](../modules/realtime-ai-validation.aps.md), RTAI-005
- [LSPNAV](../modules/lsp-graph-navigation.aps.md)
- [Approved design](../specs/2026-07-20-lsp-graph-backed-references.md)
