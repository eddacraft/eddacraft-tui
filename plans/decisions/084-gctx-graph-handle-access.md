# ADR-084: GCTX graph-handle access — daemon-RPC with daemon-side egress projection

## Status

**Accepted** — 2026-06-15, Josh. Synthesised by a planning council
(`plan-f211c211`; architect + kernel-maintainer + adversarial-reviewer) at the
owner's request, after GCTX Phase 0 (the
[GCTX-001 projection contract](../../docs/architecture/graph-context-delivery-spec.md)
Merged via #2628 and GCTX-002 Merged via #2619) closed. Resolves the open
architectural prerequisite that left the GCTX Phase 1 tool items (GCTX-010..013,
021..023, 030) Draft: **how `anvil mcp serve` obtains a graph handle.** The
Phase 1 items may flip to Ready once GCTX-010's binding conditions C1–C5 are
folded into item text. Owner refinement at acceptance: the Phase-2 isolation
layer is a **same-process second service surface over the same graphs** (option A
below), not a dedicated daemon with its own graph.

## Date

2026-06-15

## Context

[ADR-083](083-gctx-mcp-delivery-target.md) (Accepted) fixed the GCTX delivery
target as the Rust `anvil mcp serve` (RMCPF) surface. The
[GCTX-001 contract](../../docs/architecture/graph-context-delivery-spec.md) fixed
the egress *rules* (identity-only default, sealed egress DTO, single
`GctxProjector` choke point, CE-1..CE-12 from the
[PV-9 egress review](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)).
Neither answered the wiring question the Phase 1 tools depend on: the MCP server
holds no graph, so where does an identity-only graph query actually run?

Code facts established by the council (cite as given):

- **The MCP server is stateless and graph-less.** `anvil mcp serve` (`crates/anvil-cli/src/commands/mcp.rs`) is a line-oriented stdio JSON-RPC handler. It knows no workspace root at startup; every tool call receives `workspaceRoot` as a validated argument and holds no resident graph (only an auth cache + a shared tokio runtime).
- **The daemon already holds the live graphs.** `anvil-intercept` keeps a per-worktree warm `KernelGraphCache` of `(SymbolGraph, DependencyGraph)` (`crates/anvil-intercept/src/kernel_cache.rs`); `TrustGraph` is built on demand; `GraphRegistry::new(&semantic, &dependency, &trust)` (`crates/anvil-graph-cache/src/registry.rs`) is the GV2-020 read surface, with a `background_read()` tier — exactly the GV2-023 "context projection" read class GCTX is mapped to.
- **The daemon has an extensible RPC and a readiness signal.** RPC is NDJSON over a Unix socket / Windows named pipe; methods are `anvil/*` consts + request/response structs in `anvil-intercept-proto/src/protocol.rs`, dispatched in `anvil-intercept/src/ipc.rs`. The existing read-only `anvil/workspace_status` returns `WorkspaceAssurance { state: AssuranceState (Clean/Stale/Pending/Running/Unavailable), reason, generation }` — the CE-7 degradation signal, already computed.
- **Fresh per-call construction is expensive and ill-fitting.** Building the graph in the MCP process would need tree-sitter parser deps in the MCP crate and an O(workspace) cold parse *on every call*, with no snapshot to load (ADR-069 Sub-phase B is unshipped and default-off). It also cuts against the parser-free daemon boundary (ADR-064). The existing "embedded fallback" tools (`check.rs`/`suppress.rs`) do **not** build a graph, so they are no precedent for daemon-less graph access.

The product owner set the posture before the council: **daemon-required, degrade
gracefully** (not daemon-less); **thin vertical slice first**; and a Phase-2
direction to isolate assistant query-serving from the enforcement hot path. At
acceptance the owner refined that direction: rather than maintain a second graph
(in-process or in a separate daemon), add a **second service surface that reads
the same graphs** — see option A in the Decision.

## Decision

**GCTX tools obtain graph data by querying the running `anvil-intercept` daemon
over a new read-only `anvil/gctx/*` RPC; the daemon performs the egress
projection and returns sealed DTOs. The MCP server never holds or builds a
graph.**

Four parts:

1. **Daemon-RPC, daemon-required, degrade gracefully.** GCTX tools call the
   daemon; they never construct a graph in-process. Readiness is carried by the
   existing `WorkspaceAssurance` embedded in every GCTX response: `Clean` → full
   identity projection; `Stale` → identity results served, marked stale (the PV-9
   CE-7 identity-only carve-out); `Pending`/`Running` (warming) →
   `GctxError::NotReady`, empty results; daemon absent (socket unanswered) →
   `GctxError::GraphUnavailable`. **No fresh per-call construction, no whole-file
   fallback** (CE-7).

2. **Projector home = daemon-side. The RPC response *is* the sealed egress DTO.**
   The single `GctxProjector` choke point (CE-5) runs in the daemon, which already
   holds the live `GraphRegistry`, the resident source, and the
   `anvil-intercept-rules` secret detector. This keeps redaction **upstream of the
   wire**: for
   the Phase-2 snippet surface, secret-scanning/path-filtering must happen before
   any source text leaves the daemon, so the choke point cannot live in the
   consumer. The MCP crate receives already-sealed DTOs and **never links graph
   internals**.

3. **Crate split** (resolving the wire-crate-pollution and no-leak-test-placement
   concerns the council raised against a naive daemon-side design):
   - **`anvil-gctx-types`** (new leaf) — the sealed value DTOs (`SymbolSummary`,
     `GctxError`, `RedactionSummary`, `OpaqueCursor`; later `SnippetResult`) +
     serde + the **CE-5 structural no-leak test**. Depends on `anvil-kernel-types`
     (for `SymbolIdentity`) + serde **only — no graph-cache**. Depended on by both
     `anvil-intercept-proto` (wire) and `anvil-cli` (MCP consumer). Because this
     crate *cannot compile* against `GraphDelta`/`SymbolNode`, the no-leak
     guarantee is enforced by the Cargo graph, not by convention.
   - **`anvil-gctx-egress`** (new) — the `GctxProjector` that builds those DTOs
     from `GraphRegistry::background_read()` and runs CE-2/CE-3 redaction. Depends
     on graph-cache + intercept-rules + gctx-types. **Daemon-only.**
   - **`anvil-intercept-proto`** — new `anvil/gctx/*` request/response wire structs
     referencing the `anvil-gctx-types` value DTOs; stays graph-cache-free.
   - **`anvil-cli`** (MCP) — links `anvil-gctx-types` only; deserialises sealed
     responses; runs the tool handler with CE-8 workspace-root validation; stdio
     only.

4. **GCTX is its own service surface over the same graphs, not the enforcement
   path.** Phase 1 handles `anvil/gctx/*` via a separate read-only `GctxDispatch`
   (its own dispatch arm in `ipc.rs`), **not** the save-time `SaveTimeDispatch`
   trait, so GCTX queries never sit on the enforcement hot path. Phase 2 promotes
   this to a **same-process second service surface** (option A — see below): a
   dedicated GCTX listener/socket + its own read worker pool inside the **same**
   daemon process, reading the **same** in-memory graphs — never a second daemon
   with its own graph, so there is exactly one substrate and one truth.

   **Read-concurrency model (accepted).** GCTX reads must not contend with
   save-time mutation. The save-time path publishes an **immutable read snapshot**
   of the graph after each applied delta (`arc-swap` of an `Arc<GraphSnapshot>`);
   the GCTX service reads the latest published snapshot **lock-free**, never
   acquiring the `KernelGraphCache` write `Mutex`. This supersedes the
   snapshot-under-lock mitigation in C2 below: the writer is never blocked by a
   reader, and a reader always sees a consistent point-in-time graph. The
   snapshot's freshness/generation feeds the CE-7 `WorkspaceAssurance` marker.

**Phase boundary.** Phase 1 (thin slice) pilots **`anvil_search_symbols`
(GCTX-010, identity-only — no snippet/CE-1/CE-2 escalation)** end-to-end over the
*existing* intercept daemon via the separate `GctxDispatch`, building the reusable
spine (the two crates, the projector, the no-leak test,
CE-3/CE-4/CE-6/CE-7/CE-10/CE-11). **Phase 2** adds the owner's isolation layer —
the **same-process second GCTX service surface over the same graphs** (option A),
with the lock-free read-snapshot above — plus all snippet-bearing surfaces. The
`anvil/gctx/*` contract is stable across the boundary, so promoting Phase 1's
dispatch arm to a dedicated service endpoint does not change the MCP consumer.

   **Option A (chosen) vs option B.** A = same-process second service surface
   over the shared graphs (chosen): no duplication, one substrate, one egress
   choke point. B = a separate process sharing the graphs was rejected — live
   `petgraph` structures are not shareable across processes without shared-memory
   arenas (complex) or a proxy hop into the owning daemon (latency for little
   isolation gain), and it reintroduces the divergence/duplication risks A avoids.

## Rationale

- **The daemon is the only place a warm graph already exists.** Reusing it avoids
  parser deps in the MCP crate, an O(workspace) per-call parse, and a second graph
  implementation to keep correct and redact.
- **Daemon-side projection gives the strongest CE-5 guarantee.** The MCP crate
  links only `anvil-gctx-types` (graph-free), so it is *structurally incapable* of
  emitting an internal type. Redaction stays upstream of the wire — decisive for
  the Phase-2 snippet case, where MCP-side projection would ship un-redacted source
  across the socket.
- **The crate split answers the counter-position.** The kernel-maintainer argued
  for MCP-side projection to keep the wire-frozen proto crate clean and to bind the
  no-leak test at the consumer. Splitting the pure value types (`anvil-gctx-types`,
  shared, hosts the no-leak test) from the projector logic (`anvil-gctx-egress`,
  daemon-only) keeps proto graph-free *and* binds the no-leak test where the
  egress types are defined — capturing both positions.
- **It realises the owner's Phase-2 direction cleanly.** A stable sealed-DTO RPC
  contract makes the service surface promotable in place: a `GctxDispatch` arm on
  the existing socket today, a dedicated same-process GCTX listener + read worker
  pool over the *same* graphs tomorrow — with no change to the MCP consumer and no
  second graph to keep in parity.
- **Reuse `WorkspaceAssurance` for CE-7.** The degradation signal already exists;
  GCTX embeds it rather than inventing a parallel readiness notion.

## Consequences

**Positive.** No graph in the MCP process; one redaction choke point upstream of
the wire; MCP crate cannot leak internal types (Cargo-enforced); CE-7/CE-8 ride
existing daemon machinery. The Phase-2 isolation layer (option A) adds query-path
isolation with **one** substrate and **one** egress choke point — no second graph,
no second watcher, no divergence.

**Negative / accepted.** GCTX becomes daemon-dependent; the daemon gains a
read-only egress-projection responsibility (bounded to `anvil-gctx-egress`, off
the hot path) and, in Phase 2, a second service listener + read worker pool in the
same process. Two new crates. The lock-free read snapshot adds a per-delta publish
cost on the save path (an `Arc` swap of an immutable snapshot) — cheap, and off
the critical-section hold.

Because GCTX is daemon-required, its usefulness depends on the daemon actually
running. Today the save-time daemon has no auto-start and falls back silently
(DSV-021), so the Phase-1 slice leans on
[daemon-lifecycle (DLIFE)](../modules/daemon-lifecycle.aps.md) making
daemon-backed protection the normal user path. Sequence GCTX-010's rollout
against DLIFE so the slice does not ship into an environment where the daemon is
still opt-in (where every GCTX query returns `GraphUnavailable`).

**Binding conditions on GCTX-010 before it flips to Ready** (the council's
critical findings — these gate the Phase-1 item, not this ADR):

- **C1 — Cold-start warm-up (enough triggers).** The daemon's cache is
  *save-populated*: a fresh session has no graph for an unedited worktree, so
  `anvil_search_symbols` would return empty until the user saves. Silent empty
  results are a product-death failure. GCTX-010 MUST ensure the graph warms
  through a *sufficient set* of triggers, not a single hook — at minimum: a
  warm-up on MCP session init via the existing `anvil/request_full_scan` verb;
  an on-demand warm-up when a GCTX query hits a cold/`Pending` worktree; and a
  structured `GctxError::NotReady` + recovery-hint enum the assistant can act on
  while warming. Re-warm on workspace-root change / cache eviction. The bar is
  that a realistic first-use session reaches a useful graph without the user
  having to manually save files first. (The Phase-2 service surface does not by
  itself warm the graph — these triggers are needed regardless of provider.)
- **C2 — No hot-path coupling on reads.** GCTX reads MUST NOT block save-time
  mutation. Phase 2 satisfies this with the accepted **lock-free read snapshot**
  (save-time publishes an immutable `Arc<GraphSnapshot>` via `arc-swap`; GCTX
  reads the latest without taking the write `Mutex`). For the Phase-1 dispatch-arm
  pilot before that snapshot exists, the projector MUST take a cheap copy of the
  matched entries under the cache lock and release it *before* filtering/
  pagination — holding the inner `Mutex` across the whole projection is prohibited
  (ADR-031 80ms p95 gate). The Phase-2 per-delta snapshot publish is itself
  net-new cost on the save hot path (`arc-swap` is not yet a workspace
  dependency): it MUST be bench-gated against the ADR-031 80ms p95 budget, and
  `GraphSnapshot` MUST publish via `Arc`-shared sub-structures (no full deep clone
  of the graph per delta). The "cheap" claim is unproven until measured.
- **C3 — Daemon-side root admission (CE-8).** The daemon MUST validate the
  client-supplied `workspace_root` against the connection's admitted-root set
  (reuse the `SaveTimeConn` admission gate) before projecting — a hostile MCP
  client can send an arbitrary or sibling-worktree root; client-side validation
  alone is insufficient.
- **C4 — `gctx.egress` flag sequencing (CE-9 / FLAGCAT).** The `gctx.egress`
  manifest entry MUST land in the PR that adds its Rust consumer gate *and* a TS
  consumer reader, or the FLAGCAT orphan-flag drift gate fails. (Phase 1 is
  identity-only, so the flag gates the Phase-2 snippet path; sequence accordingly.)
- **C5 — Snippet secret-scan completeness (Phase 2).** The SCAN-002 per-line
  length guard (4 KiB) silently skips long lines. Note this guard lives in the
  `anvil-checks` secret scanner (`crates/anvil-checks/src/secret/types.rs`), *not*
  on `SecretDetectionRule` (`anvil-intercept-rules`); GCTX-010's snippet phase
  MUST first confirm which secret-scan path the daemon-side projector invokes and
  that the guard is in force on it. On that path, a skipped line MUST be treated
  as a detector error and redacted (CE-2 fail-closed), and byte ranges expanded to
  line boundaries before scanning.

**Open / deferred.** MCP↔daemon connection liveness vs the 60s idle timeout, and
whether CE-6 per-session credits are connection- or session-token-scoped
(a reconnect must not reset the budget); confirm `anvil-intercept-rules` is
already in the daemon dep closure (it is, via save-time) so the egress crate adds
no new transitive parser deps.

## Alternatives considered

- **Fresh per-call graph construction in the MCP process.** Rejected: needs parser
  deps in the MCP crate + O(workspace) cold parse every call, no snapshot to load,
  cuts against ADR-064. The owner ruled it out.
- **MCP-side projection** (daemon returns structural data, MCP projects).
  Rejected as the default: ships un-redacted material across the wire (fatal for
  the snippet phase) and makes the provider less pluggable. Its valid concerns
  (proto cleanliness, no-leak-test placement) are folded into the crate split.
- **Hybrid (daemon-RPC primary, cold-build fallback).** Deferred: doubles the
  graph surface (two paths to keep correct and redact) and front-loads the
  expensive path. Superseded by the Phase-2 same-process service surface over the
  shared graphs, which gives query-path isolation without a second graph or an
  in-process cold-build fallback.
- **Separate GCTX process sharing the graphs (option B).** Rejected: live
  `petgraph` structures are not shareable across processes without shared-memory
  arenas (complex) or a proxy hop into the owning daemon (latency for little
  isolation gain), and it reintroduces the duplication/divergence risks the
  same-process surface (option A) avoids.

## References

- [ADR-083](083-gctx-mcp-delivery-target.md) — GCTX MCP delivery target (RMCPF)
- [ADR-075](075-v080-graph-product-scope.md) — v0.9 GCTX scope + entry gates
- [ADR-064](064-intercept-graph-cache-crate-boundary.md) — parser-free daemon / crate boundary
- [ADR-069](069-graph-v2-persistence.md) — graph snapshot persistence (Sub-phase B, unshipped)
- [ADR-031](031-validation-latency-rubric.md) — save-time latency budget
- [GCTX-001 projection contract](../../docs/architecture/graph-context-delivery-spec.md) — CE-1..CE-12, sealed egress DTO, `GctxProjector`
- [PV-9 context-egress privacy review](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
- [graph-v2-foundation-spec.md](../../docs/architecture/graph-v2-foundation-spec.md) — GV2-020 registry + GV2-023 consumer query contract
- [`graph-context-delivery.aps.md`](../modules/graph-context-delivery.aps.md) — GCTX module + work items
- [daemon-lifecycle (DLIFE)](../modules/daemon-lifecycle.aps.md) — makes daemon-backed protection the normal user path; GCTX's daemon-required posture depends on it
- Planning council session: `plan-f211c211`
