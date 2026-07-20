# LSP Graph-Backed References Design

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Authoritative | LSPNAV | Proposed | Approved design from Planning Council `plan-33b005f5`, 2026-07-20 |

| Upstream | Downstream |
| -------- | ---------- |
| ADR-109, ADR-111, GCTX CE-5/CE-11 contracts, RTAI-005 | LSPNAV work items and execution plans |

## Goal

Provide exact, complete and bounded `textDocument/references` for one certified
language tier without expanding RTAI-005 beyond production diagnostics or
weakening the daemon's workspace, privacy, latency, and save-time guarantees.

## Ownership

| Boundary | Owner | Responsibility |
| -------- | ----- | -------------- |
| Mid-edit diagnostics | RTAI-005 | LSP lifecycle, debounce and diagnostic publication over `scan_buffer` |
| Graph storage | `anvil-graph-cache` | Occurrence structures, deltas, composite snapshot invariants and persistence |
| Rebuild/publication | Intercept | Held anchors, parsing, scheduling, rebuilds and atomic snapshot publication |
| Query/egress substrate | GCTX | Anchored query, daemon-derived UTF-16 coordinates, CE-5 outcomes and CE-11 control |
| Editor projection | LSPNAV | URI and LSP DTO mapping, dynamic registration, client lifecycle and LSP error mapping |

The first release is exact references only. Impact-of-change and affected-test
discovery remain separate Proposed intents and introduce no v1 method names or
wire contracts.

## Accuracy Contract

### Certified tier

V1 selects one language by evidence. Certification is against a closed,
published occurrence taxonomy that covers declarations and every statically
resolved use the tier claims to support. At minimum the corpus distinguishes
definitions/declarations, imports and re-exports, aliases, overloads, nested and
shadowed scopes, reads, writes, calls, type uses, multiple uses in one container,
comments/strings, and generated or unresolved constructs.

The certification gate requires:

- adjudicated golden fixtures plus differential results against the language's
  authoritative/native reference engine;
- identical atomic results after full build, incremental update, persisted
  restore, and daemon restart;
- exact URI and UTF-16 ranges, including `includeDeclaration` behaviour;
- negative evidence for unsupported syntax, dirty buffers, generation races,
  overflow, budget exhaustion, busy, cancellation, deadline and disablement;
- Unix and Windows transport evidence;
- real-client tests for every supported launch client.

A native engine is evidence, not sole authority: every difference is classified
against the versioned taxonomy and committed golden corpus.

### Capability lifecycle

The `initialize` response does not advertise a global references provider.
After `initialized`, the server dynamically registers references only when all
of these are true:

1. CE-11 permits reference egress;
2. the client supports dynamic references registration;
3. its language/document selector matches a certified tier;
4. the server process holds the matching certification identity.

The matrix is frozen for the process. Unsupported requests fail with
`TierNotCertified`; they do not return `null`. A correctness invariant failure
cancels affected requests, unregisters the tier, and requires recertification
before it can be advertised again.

The LSP process maintains an authenticated, bounded latest-value subscription to
the daemon's monotonic navigation-capability epoch. The initial snapshot and
coalesced changes contain enabled/disabled state plus certification identities,
never paths or source. Disconnect, daemon restart, epoch change, or tier
invalidation cancels in-flight navigation and unregisters the provider. The
process registers again only after a fresh enabled, certified snapshot.

## Snapshot Model

The only readable generation unit is an immutable composite `GraphSnapshot`.
It contains:

- symbol, dependency, and call graphs;
- a deterministically ordered occurrence index;
- per-file indexed content hashes and completeness/bounds state;
- canonical root identity and monotonic graph generation;
- taxonomy, parser, extractor/resolver, persistence, coordinate, protocol, and
  certification identities.

`anvil-graph-cache` owns these structures, occurrence invariants, bounded delta
application, deterministic ordering, serialisation, and load validation. The
intercept scan coordinator owns admitted-root anchors, parsing, rebuilds,
cancellation, scheduling, and atomic publication.

Writers construct a complete next snapshot off to the side and publish it once.
Readers clone its `Arc` under a brief lock. No anchored I/O occurs while a graph
lock is held.

Persistence extends the sealed graph-cache envelope and remains disposable
acceleration state. Load accepts the whole composite or rejects it. Corrupt,
incompatible, root-mismatched, newer, or downgrade-unsupported state produces
`NotReady` and a bounded anchored rebuild. No migration or query-time partial
reconstruction is permitted.

## Query Flow

1. Apply CE-11 before query validation or graph access.
2. Resolve one admitted canonical root and validate the initiating document and
   content-hash shape.
3. Validate a bounded root-scoped open-document manifest from the authenticated
   LSP session. It carries one monotonic manifest generation and the relative
   path, LSP version, and content hash of every open document, but no source.
4. Admit the request through hierarchical work credits, the fair navigation
   queue, and non-reserved scheduler capacity.
5. Pin one composite snapshot and resolve the target plus a bounded,
   deduplicated occurrence/file worklist.
6. Reject overflow or withheld identities before anchored reads where possible.
7. Verify the initiating content hash and every participating file through the
   same held `WorkspaceAnchor`; derive explicitly tagged line/UTF-16 coordinates
   from those verified bytes. Compare every participating open document with
   the complete client manifest.
8. Recheck cancellation, deadline, and the published generation.
9. Seal one atomic CE-5 outcome; LSPNAV converts only a `Ready` result's
   relative files and tagged UTF-16 ranges to file URIs and LSP locations.

The manifest is bounded by the same immutable/lowerable request policy. An
over-cap manifest returns `Overflow`; malformed, unauthenticated, stale, or
non-root-scoped state returns `InvalidQuery`. If a participating document is
open but lacks coherent manifest state, or its client hash differs from the
indexed and anchored hash, the whole response is `ContentModified`. Dirty
buffers are therefore never queried against an on-disk occurrence generation.

One references result never crosses workspace roots. If completeness would
require another root, the request fails rather than returning a root-local
partial result.

## CE-5 Contract

| Outcome | Locations | Meaning |
| ------- | --------- | ------- |
| `Ready` | Complete, verified, workspace-relative | One exact result from one generation |
| `NotReady` | None | Complete occurrence snapshot is unavailable or rebuilding |
| `ContentModified` | None | Buffer, file hash, identity, or generation changed |
| `Cancelled` | None | Client or server cancellation won the response race |
| `Busy` | None | Fair admission or queue capacity was unavailable |
| `DeadlineExceeded` | None | Queue or execution exhausted the request deadline |
| `Overflow` | None | A public request ceiling was exceeded |
| `ReferenceBudgetExhausted` | None | Cumulative weighted credit was exhausted |
| `Disabled` | None | CE-11 disabled query egress |
| `InvalidQuery` | None | Query or root admission failed |
| `Withheld` | None | At least one required identity cannot safely egress |

Only `Ready` carries paths and ranges. They are workspace-relative and
source-free. Other variants expose stable enums and coarse retry guidance only;
they never reveal paths, URIs, symbols, source, hashes, parser details,
persistence names, sensitive counts, exact match counts, or credit balances.

LSP mappings use standard cancellation/content-modified/server-cancelled/
request-failed codes with stable, privacy-safe Anvil reason enums. V1 has no
partial-result streaming or silent truncation.

At sealing, simultaneous conditions resolve in this order: `Disabled` >
`Cancelled` > `DeadlineExceeded` > `InvalidQuery` > `NotReady` >
`ReferenceBudgetExhausted` > `Busy` > `Overflow` > `Withheld` >
`ContentModified` > `Ready`. `Cancelled` maps to LSP `RequestCancelled`,
`ContentModified` to `ContentModified`, `Busy` and `DeadlineExceeded` to
`ServerCancelled` with stable reason data, and the remaining non-ready outcomes
to `RequestFailed`. No lower-precedence condition is echoed.
Immediate fair-admission or queue-capacity refusal produces `Busy`. Once a
request is admitted, exceeding 100 ms in queue or the one-second total deadline
produces `DeadlineExceeded`.

## Capacity and Abuse Control

Checked-in ceilings bound occurrences, participating files, response bytes,
request-owned memory, workers, queue entries, and credit-account state. Runtime
configuration may lower but not raise them. Shipped successful-work defaults,
credit weights, refill, and burst limits are chosen from representative
cross-platform benchmarks and recorded with the calibration corpus and hardware
classes.

The queue deadline is 100 ms and the end-to-end deadline is one second from
request receipt. Cancellation removes queued work or cooperatively stops active
work and releases root/queue/worker/memory/scheduler permits. After enabled
admission, it wins races against every lower-precedence execution outcome and a
would-be `Ready`; a simultaneous CE-11 disable remains `Disabled` per the
normative order. Work already performed remains charged.

Save-time validation owns an exclusive scheduler reservation. Navigation uses
only the remaining class and degrades when none is safe; it never borrows the
save-time reservation.

The global navigation queue admits at most one running and one queued request
per authenticated principal/canonical root and at most one queued request per
live session. Deterministic round-robin selection runs across principals, then
roots, then sessions. A reconnect reuses the parent identities and cannot jump
ahead or multiply queue capacity.

Hierarchical weighted credits are daemon-owned and keyed by:

- authenticated OS principal/session lineage across all roots;
- that principal plus the admitted anchor's canonical root;
- a fair child lane for the live LSP session.

Client-supplied principal or path strings are never keys. Reconnect does not
restore parent credit. Monotonic refill governs all buckets; daemon restart is
the explicit reset. Expected credit is reserved atomically, actual work is
charged, and only unused reservation is refunded. Inactive bounded account
records expire deterministically.

## Rollout and Evidence

1. **Diagnostics boundary:** PR #3360 becomes production RTAI-005 diagnostics
   only, including debounce, URI/range correctness, lifecycle, daemon-down,
   Unix/Windows transport, and editor-agnostic E2E evidence.
2. **Planning:** ADR-111 and LSPNAV merge after reconciliation to the final
   diagnostics surface.
3. **Hidden model:** taxonomy, certification, occurrence deltas, composite
   snapshot, and persistence land with no query surface.
4. **Bounded RPC:** the externally reachable query boundary lands separately,
   default-disabled by CE-11.
5. **LSP projection:** dynamic certified-tier registration and exact locations
   land after substrate certification.
6. **Evidence rollout:** shadow comparison, opt-in canary, bounded soak, then
   release promotion.

Release evidence records successful and non-success rates separately and covers
p50/p95/p99/max queue, service, anchored-read, projection and total latency;
request and process memory; response size; files and occurrences; CPU,
descriptors and queue depth; cancellation quiescence; worker failure/restart;
corrupt/newer/downgrade snapshot recovery; burst and sustained abuse;
reconnect/multi-root fairness; four-agent-plus-background contention; and the
existing save-time gates on Linux and Windows.

Warm accepted queries must achieve p95 at or below 250 ms, p99 below 500 ms,
and maximum below one second on the supported reference classes. A statistically
credible save-time regression blocks promotion.

CE-11 rollback advances the capability epoch; the bounded control subscription
unregisters the provider even while idle, cancels in-flight work, disables
direct location egress, and leaves diagnostics untouched. A control disconnect
is fail-closed. Older binaries reject newer acceleration state and rebuild
without a reverse migration.

## Non-Goals

- definitions, hover, code lenses, code actions, rename, or per-keystroke graph
  rebuilds;
- impact-of-change or affected-test delivery in the first release;
- static global references advertisement;
- cross-root partial results or LSP-side workspace reads;
- source snippets or general occurrence export;
- preserving unmerged PR #3360 navigation wire shapes.

## Risks

| Risk | Control |
| ---- | ------- |
| Incomplete reference taxonomy produces plausible partial answers | Closed taxonomy, differential/golden corpus, fail-closed tier invalidation |
| Stale disk or graph generation corrupts ranges | One composite snapshot and response-set-wide anchored verification |
| Sensitive identity leaks through a failed or partial response | Closed no-location outcomes and generic `Withheld` |
| Repeated queries scrape or exhaust the daemon | Hierarchical weighted credits and bounded account state |
| Navigation delays save-time protection | Exclusive save-time reservation and contention gate |
| Platform or client capability drift | Unix/Windows plus real-client certification matrix |
| Rollback cannot read newer persistence | Reject-and-rebuild version policy; no reverse migration |

## Provenance

- Operator-approved Planning Council: `plan-33b005f5`
- Architecture decision: [ADR-111](../decisions/111-graph-backed-lsp-references.md)
- Scope boundary: [ADR-109](../decisions/109-lsp-agent-integration-reconsidered.md)
- APS module: [LSPNAV](../modules/lsp-graph-navigation.aps.md)
