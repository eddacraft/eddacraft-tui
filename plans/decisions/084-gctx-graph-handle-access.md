# ADR-084: GCTX graph-handle access — daemon-RPC with daemon-side egress projection

## Status

**Proposed** — 2026-06-15. Synthesised by a planning council
(`plan-f211c211`; architect + kernel-maintainer + adversarial-reviewer) at the
owner's request, after GCTX Phase 0 (the
[GCTX-001 projection contract](../../docs/architecture/graph-context-delivery-spec.md)
Merged via #2628 and GCTX-002 Merged via #2619) closed. Resolves the open
architectural prerequisite that left the GCTX Phase 1 tool items (GCTX-010..013,
021..023, 030) Draft: **how `anvil mcp serve` obtains a graph handle.** Awaiting
owner acceptance before the Phase 1 items flip to Ready.

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
direction — rather than maintain a second in-process graph path, **lazily spawn a
dedicated assistant/GCTX graph daemon** (a named feature) only when GCTX is
actually used.

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
   The single `GctxProjector` choke point (CE-5) runs in the daemon, where the
   live `GraphRegistry`, the resident source, and the `anvil-intercept-rules`
   secret detector already are. This keeps redaction **upstream of the wire**: for
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

4. **GCTX is its own dispatch, not the enforcement path.** The `anvil/gctx/*`
   methods are handled by a separate read-only `GctxDispatch` (its own dispatch
   arm in `ipc.rs`), **not** added to the save-time `SaveTimeDispatch` trait, so
   GCTX queries never sit on the enforcement hot path and the trait's "save-time"
   semantics stay coherent.

**Phase boundary.** Phase 1 (thin slice) pilots **`anvil_search_symbols`
(GCTX-010, identity-only — no snippet/CE-1/CE-2 escalation)** end-to-end over the
*existing* intercept daemon, building the reusable spine (the two crates, the
projector, the no-leak test, CE-3/CE-4/CE-6/CE-7/CE-10/CE-11). Phase 2 adds the
owner's **lazily-spawned dedicated assistant graph daemon** as a *pluggable
provider behind the same `anvil/gctx/*` contract* (the MCP consumer never
changes), plus all snippet-bearing surfaces.

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
  contract makes the warm-graph provider pluggable: the enforcement daemon today,
  a dedicated spawned assistant daemon tomorrow, with no change to the MCP
  consumer.
- **Reuse `WorkspaceAssurance` for CE-7.** The degradation signal already exists;
  GCTX embeds it rather than inventing a parallel readiness notion.

## Consequences

**Positive.** No graph in the MCP process; one redaction choke point upstream of
the wire; MCP crate cannot leak internal types (Cargo-enforced); the provider is
pluggable for Phase 2; CE-7/CE-8 ride existing daemon machinery.

**Negative / accepted.** GCTX becomes daemon-dependent; the daemon gains a
read-only egress-projection responsibility (bounded to `anvil-gctx-egress`, off
the hot path). Two new crates.

**Binding conditions on GCTX-010 before it flips to Ready** (the council's
critical findings — these gate the Phase-1 item, not this ADR):

- **C1 — Cold-start warm-up.** The daemon's cache is *save-populated*: a fresh
  session has no graph for an unedited worktree, so `anvil_search_symbols` would
  return empty until the user saves. GCTX-010 MUST trigger a warm-up on MCP
  session init via the existing `anvil/request_full_scan` verb (or surface a
  structured `GctxError`/recovery-hint enum the assistant can act on) — silent
  empty results are a product-death failure. This is also the strongest argument
  for pulling the Phase-2 dedicated daemon (which can run a background full scan)
  forward if the warm-up trigger proves insufficient.
- **C2 — Bounded lock hold.** The `GctxProjector` MUST snapshot the matched graph
  entries under the cache lock and release it *before* filtering/pagination, so
  GCTX query latency never couples to the save-time hot path (ADR-031 80ms p95
  gate). Holding the inner `Mutex` across the whole projection is prohibited.
- **C3 — Daemon-side root admission (CE-8).** The daemon MUST validate the
  client-supplied `workspace_root` against the connection's admitted-root set
  (reuse the `SaveTimeConn` admission gate) before projecting — a hostile MCP
  client can send an arbitrary or sibling-worktree root; client-side validation
  alone is insufficient.
- **C4 — `gctx.egress` flag sequencing (CE-9 / FLAGCAT).** The `gctx.egress`
  manifest entry MUST land in the PR that adds its Rust consumer gate *and* a TS
  consumer reader, or the FLAGCAT orphan-flag drift gate fails. (Phase 1 is
  identity-only, so the flag gates the Phase-2 snippet path; sequence accordingly.)
- **C5 — Snippet secret-scan completeness (Phase 2).** The `SecretDetectionRule`
  line-length guard (SCAN-002, 4 KiB) silently skips long lines; for snippet
  egress a skipped line MUST be treated as a detector error and redacted
  (CE-2 fail-closed), and byte ranges expanded to line boundaries before scanning.

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
  expensive path. The owner's Phase-2 spawned-daemon supersedes the need for an
  in-process cold-build fallback.

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
- Planning council session: `plan-f211c211`
