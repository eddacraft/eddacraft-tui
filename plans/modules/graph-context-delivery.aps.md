# Graph Context Delivery

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| GCTX | —     | In Progress | 9/14 |

**Last reviewed:** 2026-06-23 (Phase 0 — Delivery Contract — complete. **GCTX-001 (projection contract) Merged 2026-06-15 via #2628** — the spec [`graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md) folds the context-egress privacy review (PV-9) conditions CE-1..CE-12 onto the GV2-023 consumer query contract. **GCTX-002 (MCP delivery target) Merged 2026-06-15 via #2619** — discharged by [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted** (Rust RMCPF `anvil mcp serve` surface); RMCPF defers GCTX work by design, so no edit to rust-mcp-full-port. Module **In Progress, 9/14** (GCTX-010 pilot Merged 2026-06-16 via #2657; GCTX-011 `find_dependents` Merged 2026-06-16 via #2685; GCTX-012 `anvil_impact_of_change` Merged 2026-06-17 via #2693; **GCTX-013 `anvil_affected_tests` Merged 2026-06-17 via #2700** — test attribution + coverage gaps over the same spine, no new substrate; reuses GCTX-012's `is_test_file` + the dependency graph's forward `dependencies_of` edges for evidence; **GCTX-014 `anvil_find_callers` Merged 2026-06-17 via #2715** — symbol-level caller traversal projecting the GCALL-003 `callers_of` read API, completing the Phase 1 tool surface (010..014); **GCTX-030 (`graph://` MCP resources) Merged 2026-06-18 via #2772** — the read-only `graph://stats`/`symbols`/`edges` resource surface, identity-only, with CE-6 pagination and a `bounded` edges flag; **GCTX-020 Done 2026-06-20** — parser-free conservative token estimator in `anvil-graph-cache`, with deterministic fixed-corpus and input-cap tests). With the Phase 1 tool queue + resource surface complete and GCTX-020 done, the Phase-2 snippet items (021..023) are **promoted Draft → Ready 2026-06-23** with the PV-9 snippet gates folded into item text and the substrate prerequisite filed as **[GV2-032](graph-v2-foundation.aps.md)** (span + per-file content-hash producer); all build on the CE-5 sealed egress DTO + `GctxProjector` + structural no-leak spine that GCTX-010 established, using the daemon-RPC graph-handle path settled by ADR-084.)

**Readiness update 2026-06-23:** GCTX-021..023 are now **Ready**. The
CE-1/CE-2/CE-3/CE-5/CE-6/CE-7/CE-9/CE-11/CE-12 snippet gates are written into the
item text below, and the one true blocker — the resident `SymbolNode` carries no
source span and the graph records no per-file content hash, so a projector cannot
locate or freshness-check a snippet — is filed as the substrate prerequisite
**[GV2-032](graph-v2-foundation.aps.md)** (Ready). All source-text handling runs
daemon-side in `anvil-gctx-egress` through the single CE-5 `GctxProjector`; the
`gctx.egress` manifest flag (CE-9, deferred from GCTX-010 C4) lands with GCTX-021.
GCTX-020 (`estimate_gctx_tokens`) remains the non-egress estimator GCTX-022
consumes.

> **Scoped to v0.9, not v0.8.0-beta (2026-06-08, [ADR-075](../decisions/075-v080-graph-product-scope.md),
> Accepted via council).** GCTX was considered for the v0.8.0 window but the
> council recommended against it: 0/13, unproven, with an unresolved GCTX-002
> architectural decision (which MCP target) and an unmet **context-egress privacy
> review** — the 2026-06-08 GV2 privacy verdict covers persistence only (PV-9
> reserves export surfaces for a separate review). GCTX opens the **v0.9** window
> alongside the non-critical-path GV2 items (registry/contracts); the egress
> privacy review is a v0.9 cut prerequisite.

> **Entry gates landed (2026-06-15).** Both ADR-075 entry decisions are now
> resolved: [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) (Accepted)
> fixes the MCP delivery target as the Rust `anvil mcp serve` (RMCPF) surface,
> and the [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
> (APPROVE-WITH-CONDITIONS, 4/4) discharges the egress-privacy prerequisite. Its
> conditions **CE-1..CE-12** fold into GCTX-001 (contract) and the named per-item
> targets; **CE-1** (snippet egress opt-in, identity-only default) and **CE-5**
> (sealed egress DTO + single redaction choke point + structural no-leak test)
> are hard gates that must be written into item text before the snippet/Phase-1
> items flip to Ready.

## Purpose

Expose Graph v2 as useful, bounded context for AI coding assistants without
turning Graph v2 into an agent-context feature.

**Why:** GV2 gives Anvil a persistent, joined structural model for enforcement,
trust, control, and provenance. Agents can produce better results when they tap
that same deterministic substrate instead of rereading whole files or guessing
from raw text. GCTX is the delivery projection: MCP tools, resources, context
slicing, affected-test lookup, and token-reduction measurement built on top of
GV2.

**Framing rule:** Graph v2 is Anvil-first. If assistant context delivery and
enforcement/provenance requirements conflict, GV2 wins and this module adapts.

## In Scope

- Assistant-facing graph query tools over the GV2 query contract
- Symbol search, caller/dependent traversal, impact-of-change, symbol context,
  and affected-tests projections
- Context slicing that returns deterministic snippets within a token budget
- Token estimation and token-reduction benchmarks against naive file-reading
  baselines
- MCP resources that expose safe graph summaries and stats
- User guide for Claude Code, Cursor, Continue, Zed, and similar clients
- Coordination with the Rust MCP full port when the assistant-facing surface
  moves from the TS server to `anvil mcp serve`

## Out of Scope

- Defining Graph v2 schemas, stable IDs, graph deltas, hot-path indexes, or
  persistence strategy; owned by GV2
- Launch-critical pre-write validation; owned by RMCP
- Full MCP parity or TS MCP server retirement; owned by RMCPF
- Making MCP the primary control plane
- Community detection / Leiden clustering
- Embedding-based semantic search
- Visual graph UI surfaces; dashboard modules own visualisation
- Multi-repo graph registry
- Hot-path daemon enforcement reads

## Interfaces

**Depends on:**

- GV2 — graph taxonomy, stable identity, deltas, persistence, query traits, and
  hot/non-hot path boundary
- RMCPF — eventual Rust MCP parity surface for existing server functionality
- `archive/anvil-mcp-server` — interim delivery surface until RMCPF lands
- `anvil-kernel` / `anvil-kernel-types` — graph query implementation and
  diagnostic/source-span types
- RTAI/RMCP — release launch path, kept separate from graph context delivery

**Exposes:**

- Assistant-facing graph query tools:
  - `anvil_search_symbols`
  - `anvil_find_callers`
  - `anvil_find_dependents`
  - `anvil_impact_of_change`
  - `anvil_symbol_context`
  - `anvil_affected_tests`
- MCP resources:
  - `graph://symbols`
  - `graph://edges`
  - `graph://stats`
- Context-slicing and token-budget utilities
- Token-reduction benchmark harness

## Constraints

- UK English spelling in all plan text and user-facing docs
- Context slices must be deterministic for identical graph state and query input
- Context delivery must degrade gracefully when the graph is warming or disabled,
  and must **never** fall back to direct file reads outside the graph → redaction
  → token-budget pipeline (PV-9 review CE-7)
- The default egress surface is **identity-only** (symbol names, kinds,
  workspace-root-relative paths, edges, structural summaries); returning source
  **text** (snippets) is opt-in, default-off behind the `gctx.egress` flag, and
  emitted only through a single sealed-DTO redaction choke point that runs
  deny-by-default secret scanning and sensitive-path filtering (PV-9 review
  CE-1/CE-2/CE-3/CE-5). Sensitive diagnostics, secret content, and private
  provenance fields are redacted by default before crossing MCP boundaries; the
  full conditions are recorded in the
  [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
- MCP tools are additive and must not break existing tool contracts
- This module must not introduce schema requirements that belong in GV2
- Benchmarks must be reproducible and checked in before marketing claims are made

## Prerequisites

- GV2-001 graph taxonomy accepted
- GV2-020 multi-graph registry/query traits available
- GV2-023 consumer query contract accepted
- RMCP shipped or excluded from graph-context docs so launch users are not asked
  to install two overlapping MCP paths
- RMCPF scope known if this module targets the Rust MCP server directly

## Ready Checklist

Module promoted to **Ready** 2026-06-15 (both ADR-075 entry gates landed):

- [x] GV2 query contract exposes the graph reads this module needs — GV2-023
      consumer query contract authored 2026-06-15 in
      [`graph-v2-foundation-spec.md`](../../docs/architecture/graph-v2-foundation-spec.md)
      ("The consumer query contract (GV2-023)"); GCTX's mapped scenario is the
      identity-only impact-set projection through the `GctxProjector` choke point
      (PV-9 CE-5), with source-text egress gated behind `gctx.egress` (CE-1)
- [x] MCP delivery target decided: interim TS server, Rust RMCPF server, or both —
      [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) Accepted (Rust RMCPF
      `anvil mcp serve`)
- [x] Redaction rules for graph context are reviewed by security —
      [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md),
      APPROVE-WITH-CONDITIONS (CE-1..CE-12 fold into GCTX-001)

All entry-gate readiness criteria are satisfied. The items below are
implementation-phase details owned by their named work items, **not** readiness
blockers — they are resolved during execution, not before promotion:

- Token-budget strategy agreed with MCP/server owner — GCTX-020/022 detail
- Benchmark baseline fixture set selected — GCTX-031 detail
- User-guide outline agreed with docs owner — GCTX-032 detail

---

## Work Items

### Phase 0 — Delivery Contract

#### GCTX-001: Assistant graph projection contract

- **Status:** Merged 2026-06-15 via #2628 — sole dependency GV2-023 **Merged
  2026-06-15 via #2621**, and both ADR-075 entry gates landed
  ([ADR-083](../decisions/083-gctx-mcp-delivery-target.md) Accepted +
  [PV-9 egress review](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
  filed). The contract spec is authored in
  [`graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md)
  (identity-only default, sealed egress DTO + single `GctxProjector` choke point,
  egress allowlist/residual table, CE-1..CE-12 fold). This is the contract item
  that folds CE-1..CE-12 into the spec; its CE-1 / CE-5 hard gates carry into the
  downstream Phase-1 snippet items (which stay Draft until that text lands).
- **Intent:** Define exactly which GV2 queries are safe and useful to expose to
  assistants.
- **Expected Outcome:** Contract maps assistant tasks to graph projections,
  redaction rules, warming/stale-state behaviour, pagination, and deterministic
  ordering. It **must absorb the egress conditions CE-1..CE-12** from the
  [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md):
  identity-only default with opt-in source-text egress behind `gctx.egress`
  (CE-1); deny-by-default secret scanning (CE-2) and sensitive-path / gitignore
  filtering (CE-3) on snippets; an egress field allowlist + named residual table
  that excludes GV2-013/014 fields until their own ADRs land (CE-4); a sealed
  egress DTO with a single `GctxProjector` choke point and a structural no-leak
  test (CE-5); volume bounds — quotas, opaque server-minted pagination cursors,
  per-session snippet byte ceiling, and query-param validation (CE-6); stale-graph
  snippet guard with no whole-file fallback (CE-7); session-pinned workspace root
  and stdio-only transport boundary (CE-8); enum-only telemetry (CE-10);
  kill-switch + per-response redaction summary (CE-11); and surfaced consent
  (CE-12).
- **Validation:** Review confirms no GV2 schema or hot-path enforcement contract
  is defined in this module; the contract spec includes the egress allowlist /
  residual table and the sealed-DTO + no-leak-test requirement (CE-5), which gates
  the Phase 1 tool items
- **Files:** `docs/architecture/graph-context-delivery-spec.md`,
  `docs/architecture/graph-v2-foundation-spec.md`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-023

---

#### GCTX-002: MCP delivery target decision

- **Status:** Merged 2026-06-15 via #2619 — discharged by [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted 2026-06-15** (Josh): primary target is the Rust RMCPF `anvil mcp serve` surface per RMCPF + ADR-033 parking of TS MCP; additive registration of GCTX tools/resources. Both ADR-075 entry gates are now landed (this decision + the [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)). RMCPF defers GCTX work by design (rust-mcp-full-port Out of Scope: "Creating new graph-context tools beyond what GCTX explicitly owns"), so the decision needs no edit there. Acceptance criteria CE-8 (session-pinned root; stdio-only — a networked RMCPF transport needs a new egress review before GCTX registers there) carry into implementation.
- **Intent:** Decide whether graph context tools first land on the interim TS MCP
  server, the Rust RMCPF server, or both.
- **Expected Outcome:** Decision records the target server, compatibility stance,
  and migration path so GCTX does not fight RMCP/RMCPF sequencing.
- **Validation:** Decision reviewed by RMCP/RMCPF owner and MCP server owner
- **Files:** `plans/modules/graph-context-delivery.aps.md`,
  `plans/modules/rust-mcp-full-port.aps.md`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** RMCP, RMCPF-001

---

### Phase 1 — Graph Query Tools

> **Graph-handle access fixed by [ADR-084](../decisions/084-gctx-graph-handle-access.md)
> (Accepted 2026-06-15, planning council `plan-f211c211`).** GCTX tools query the
> running `anvil-intercept` daemon over a new read-only `anvil/gctx/*` RPC
> (daemon-required; degrade via the existing `WorkspaceAssurance`/CE-7 signal — no
> per-call construction, no whole-file fallback). The CE-5 `GctxProjector` runs
> **daemon-side** (the RPC response *is* the sealed egress DTO) across two new
> crates: `anvil-gctx-types` (graph-free value DTOs + the no-leak test, shared by
> proto + MCP) and `anvil-gctx-egress` (daemon-only projector). GCTX uses its own
> `GctxDispatch`, not the save-time path. **Thin vertical slice first**:
> GCTX-010 builds this spine end-to-end. **Phase 2** = a same-process second GCTX
> service surface over the **same** graphs (option A) with a lock-free read
> snapshot (arc-swap), isolating query-serving from the hot path — not a second
> graph — plus the snippet surfaces. **GCTX-010 is Merged** (#2657; the pilot —
> C1–C5 folded as acceptance criteria, building the spine); GCTX-011/012/013,
> 021..023, and 030 stay **Draft** pending the GCTX-010 spine they inherit.

#### GCTX-010: `anvil_search_symbols` tool

- **Status:** Merged 2026-06-16 via #2657 — architecture fixed by
  [ADR-084](../decisions/084-gctx-graph-handle-access.md) **Accepted** (daemon-RPC
  + daemon-side projection); the ADR-084 binding conditions C1–C5 are folded as
  acceptance criteria below. This is the **CE-5 hard-gate item**: it builds the
  reusable spine — the sealed egress DTO crate (`anvil-gctx-types`), the single
  `GctxProjector` choke point (`anvil-gctx-egress`), and the structural no-leak
  test — that the rest of Phase 1 inherits, plus the first identity-only tool.
  Sequenced the rollout against **DLIFE** (GCTX is daemon-required; see the Phase 1
  note above). Delivered across PRs: the spine + identity tool
  (#2637), CE-6 opaque cursors (#2645), and CE-10/CE-11 telemetry + kill-switch
  (#2648), then the final slice — **C1 cold-start warm-up triggers**
  (session-init `request_full_scan` on MCP `initialize` + on-demand re-warm when a
  `search_symbols` query returns `NotReady`, both client-side and best-effort) —
  landed on top of the DSV-045 full-scan executor it relies on
  ([ADR-085](../decisions/085-daemon-full-scan-executor.md); **DSV-045 Merged
  2026-06-16 via #2674**) so the enqueue actually drives `Pending → Running →
  Clean` rather than sitting inert. The C4 `gctx.egress` **manifest** flag
  (FLAGCAT Rust+TS consumers) is deferred to Phase 2, where it gates the snippet
  path; the Phase-1 identity surface ships behind the `ANVIL_GCTX_EGRESS`
  kill-switch (CE-11) only. The daemon-side `NotReady` + recovery-hint
  degradation (CE-7) already landed with the spine.
- **Intent:** Let assistants find symbols by name, kind, file, language, and
  visibility using GV2's semantic graph projection.
- **Expected Outcome:** `anvil_search_symbols` returns paginated, deterministic,
  **identity-only** symbol summaries (`SymbolIdentity` + kind +
  workspace-root-relative path + visibility — no source text) projected
  **daemon-side** through the single `GctxProjector` over a
  `registry.background_read()` snapshot, served to `anvil mcp serve` over the new
  read-only `anvil/gctx/search_symbols` RPC on its own `GctxDispatch` (not the
  save-time path). Lands the spine: `anvil-gctx-types` (graph-free sealed DTOs +
  no-leak test) and `anvil-gctx-egress`, opaque pagination, and the CE-3 deny-list
  / CE-4 allowlist filters.
- **Acceptance criteria (ADR-084 C1–C5 + the CE gates):**
  - **CE-5 (hard gate)** — a sealed egress DTO (no `serde(flatten)`, no `PathBuf`,
    no session-local `u64` id; errors as a named `GctxError` enum), a single
    `GctxProjector` constructor, and a structural no-leak test in
    `anvil-gctx-types`; the MCP crate links only `anvil-gctx-types` and cannot
    reach graph internals.
  - **C1 — cold-start warm-up (enough triggers).** The graph is save-populated, so
    a fresh session MUST warm through a sufficient set of triggers — at minimum a
    session-init `anvil/request_full_scan`, an on-demand warm-up when a query hits
    a cold/`Pending` worktree, and a `GctxError::NotReady` + recovery-hint enum
    while warming — so a realistic first-use session is not empty.
  - **C2 — no hot-path coupling.** For the Phase-1 pilot the projector snapshots
    the matched entries under the cache lock then releases *before*
    filtering/pagination; it MUST NOT hold the `Mutex` across the projection
    (ADR-031 80ms p95). (The Phase-2 lock-free arc-swap snapshot supersedes this
    and is bench-gated separately.)
  - **C3 — daemon-side root admission (CE-8).** The daemon validates the
    client-supplied `workspace_root` against the connection's admitted-root set
    (reuse the `SaveTimeConn` gate); cross-worktree / arbitrary roots are rejected.
  - **C4 — `gctx.egress` flag sequencing (CE-9 / FLAGCAT).** The `gctx.egress`
    manifest entry lands with both a Rust consumer gate and a TS consumer reader.
    (Phase 1 is identity-only, so the flag gates the Phase-2 snippet path; sequence
    accordingly.)
  - **C5 — secret-scan path (Phase-2 guard).** Confirm which secret-scan path the
    daemon-side projector invokes; the SCAN-002 4 KiB per-line guard lives in
    `anvil-checks` (`crates/anvil-checks/src/secret/types.rs`), **not** on
    `SecretDetectionRule` — a skipped line MUST fail closed (redact). GCTX-010 is
    identity-only (no source text egresses), so this binds the snippet items.
  - **CE-3/CE-4/CE-6/CE-7/CE-10/CE-11** — sensitive-path deny-list + field
    allowlist on this enumeration surface; opaque server-minted pagination + caps;
    `WorkspaceAssurance`-driven degradation (no whole-file fallback); enum-only
    telemetry; counts-only `redaction_summary`.
- **Validation:**
  - structural no-leak test (`anvil-gctx-types`) passes — gates the build;
  - integration test queries a fixture and asserts stable `SymbolIdentity`
    ordering and opaque-cursor pagination;
  - degradation fixture: a `warming`/`disabled` graph yields a structured
    `GctxError::NotReady` / `GraphUnavailable`, never a file read (CE-7);
  - root-admission test: a cross-worktree / arbitrary `workspace_root` is rejected
    daemon-side (C3 / CE-8).
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-cli/src/mcp/tools/`,
  `flags/manifest.json` (per ADR-084)
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-001, GCTX-002, GV2-020, ADR-084 (sequence against DLIFE
  — daemon-required)

---

#### GCTX-011: `anvil_find_dependents` dependency traversal tool

- **Status:** Merged 2026-06-16 via #2685 — built directly on the GCTX-010
  spine (Merged 2026-06-16 via #2657): the sealed `anvil-gctx-types` DTOs, the
  single `GctxProjector` choke point in `anvil-gctx-egress`, and the
  `GctxDispatch` RPC surface, plus the new `anvil/gctx/find_dependents` RPC and
  the graph-free `anvil_find_dependents` MCP tool. Scoped to **dependents
  only**; symbol-level *caller* traversal was split out to **GCTX-014** and has
  since Merged via #2715 over the GCALL substrate.
- **Intent:** Let assistants inspect a symbol's local blast radius — what depends
  on it — without expensive or ambiguous whole-repo rereads.
- **Expected Outcome:** `anvil_find_dependents` returns bounded, depth-limited,
  **identity-only** traversal results over the daemon's resident dependency
  graph: for a workspace-relative file (and optional symbol identity), the
  importing files and their identity summaries, each with traversal distance,
  source file, and truncation metadata. Projected **daemon-side** through the
  single `GctxProjector` over a background-read snapshot, served to `anvil mcp
  serve` over a new read-only `anvil/gctx/find_dependents` RPC on the existing
  `GctxDispatch` (never the save-time path). Reuses the GCTX-010 spine end to
  end; no new egress crate.
- **Acceptance criteria (inherit the GCTX-010 CE spine):**
  - **CE-5** — results are a sealed `anvil-gctx-types` DTO (no `PathBuf`, no
    session-local id, no source text); the structural no-leak test covers the new
    response type. The MCP crate links only `anvil-gctx-types`.
  - **Depth bound** — traversal depth is clamped by the GV2-026
    `clamp_reverse_impact_depth` / `MAX_REVERSE_IMPACT_DEPTH` lever (no unbounded
    walk); over-depth results carry truncation metadata, never a silent cutoff.
  - **File-keyed granularity (documented limit)** — dependents resolve via the
    file-keyed `DependencyGraph::dependents_of` + `reverse_impact` reads;
    symbol-granular dependent edges are the GV2-023 freeze-target and out of scope
    here. The tool description states the file-keyed granularity so assistants do
    not over-read the result.
  - **CE-6** — opaque server-minted pagination cursor + input caps, reusing the
    GCTX-010 cursor machinery.
  - **CE-7** — `WorkspaceAssurance`-driven degradation: a warming / cold /
    `Bounded` graph yields a structured `NotReady` / `Unavailable` outcome (with
    the C1 warm-up re-warm trigger), never a whole-file fallback.
  - **C3** — the daemon validates the client `workspace_root` against the
    connection's admitted-root set (reuse the `SaveTimeConn` gate).
  - **CE-10 / CE-11** — enum-only telemetry and the `ANVIL_GCTX_EGRESS`
    kill-switch, reusing the GCTX-010 surfaces.
- **Validation:**
  - structural no-leak test for the new response DTO (gates the build);
  - fixture traversal tests cover **chain, diamond, cycle, and max-depth**
    truncation cases over a resident graph;
  - degradation fixture: a warming / `Bounded` graph yields `NotReady`, never a
    file read (CE-7);
  - opaque-cursor pagination test (stable ordering, no overlap or gap);
  - root-admission test: a cross-worktree `workspace_root` is rejected
    daemon-side (C3).
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs` (+ the `GctxDispatch` impl in
  `save_time.rs`), `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-010 (Merged 2026-06-16 via #2657), GV2-011
  (Released/Shipped v0.8.0-beta), GV2-026 reverse-impact depth lever (Merged
  2026-06-14 via #2594) — all closed.

---

#### GCTX-012: `anvil_impact_of_change` tool

- **Status:** Merged 2026-06-17 via #2693 — built directly on the GCTX-010/011
  spine: the sealed `anvil-gctx-types` DTOs, the single `GctxProjector` choke
  point in `anvil-gctx-egress` (`collect_impact` multi-source reverse-impact BFS
  + `project_impact`), the `GctxDispatch` RPC (`anvil/gctx/impact_of_change`),
  and the graph-free `anvil_impact_of_change` MCP tool. **No new graph
  substrate** — the report composes existing warm-graph reads (`symbols_in_file`
  + `dependents_of`). Contract fixed by the GCTX-001 spec
  ([`graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md)):
  paths-only input, a ≤200 input-file cap (CE-6), and an identity-only
  deterministic `ImpactReport` (affected symbols + dependent files + heuristic
  known tests); both result sets are node-budget capped with `summary.truncated`.
- **Intent:** Given a set of changed file paths (an assistant's edit set, or a
  git diff/staged path list), return the local blast radius — what changed and
  what depends on it — as one structured, assistant-readable report, so the
  assistant reasons about impact without chaining many `find_dependents` calls.
- **Expected Outcome:** `anvil_impact_of_change` accepts **changed file paths
  only** (workspace-relative; the MCP tool may derive them client-side from
  `git diff --name-only` / staged state — **never diff content**, CE-6) and
  returns a deterministic, **identity-only** `ImpactReport`, projected
  daemon-side through the single `GctxProjector` over a background-read snapshot
  and served to `anvil mcp serve` over a new read-only
  `anvil/gctx/impact_of_change` RPC on the existing `GctxDispatch` (never the
  save-time path). The report carries:
  - `affected_symbols` — identity summaries (`SymbolSummary`) of the symbols
    **defined in the changed files** (the change surface), deterministically
    ordered;
  - `dependent_files` — the depth-bounded reverse-impact closure of the changed
    set (the union of GCTX-011's `collect_dependents` over each input file,
    file-keyed with traversal distance, deduplicated across inputs);
  - `known_tests` — the subset of `dependent_files` whose paths match a
    **best-effort, explicitly-heuristic** test-file convention (e.g.
    `*.test.*` / `*.spec.*` / `_test.rs` / `tests/` / `__tests__/`), marked
    heuristic so an assistant does not treat it as authoritative coverage —
    GCTX-013 `anvil_affected_tests` owns the richer evidence-edge + coverage-gap
    treatment;
  - counts-only `redaction_summary` / metadata (input file count, omitted/
    truncated markers when the input cap or the GV2-026 depth/result caps bind).
- **Acceptance criteria (inherit the GCTX-010/011 CE spine):**
  - **CE-5** — the report is a sealed `anvil-gctx-types` DTO (no `PathBuf`, no
    session-local id, no source text); the structural no-leak test covers the new
    `ImpactReport` type. The MCP crate links only `anvil-gctx-types`.
  - **CE-6** — a hard **≤200 changed-file input cap** plus per-path validation
    (≤512 bytes, no NUL, no absolute path, no `..`, no scheme prefix) **before**
    any graph read, reusing the GCTX-011 validation helpers. Diff input is
    **paths-only** — the daemon RPC never receives or forwards diff content.
  - **Depth bound** — the `dependent_files` closure is clamped by the GV2-026
    `clamp_reverse_impact_depth` / `MAX_REVERSE_IMPACT_DEPTH` lever and the
    `collect_dependents` node budget (no unbounded walk); over-bound results
    carry truncation metadata, never a silent cutoff.
  - **CE-7** — identity-only carve-out: the report **may continue** under the
    existing warming/stale degradation (PV-9 CE-7 grants this for identity-only
    responses); a cold / unavailable graph yields a structured
    `NotReady` / `Unavailable` outcome (with the C1 re-warm trigger), never a
    whole-file fallback.
  - **C3** — the daemon validates the client `workspace_root` against the
    connection's admitted-root set (reuse the `SaveTimeConn` gate).
  - **CE-10 / CE-11** — enum-only telemetry and the `ANVIL_GCTX_EGRESS`
    kill-switch, reusing the GCTX-010 surfaces.
  - **Determinism** — for an identical changed-file set and graph state the
    `ImpactReport` is byte-identical (sorted sections, no map-iteration order
    leakage), matching the spec's determinism guarantee.
- **Validation:**
  - structural no-leak test for the new `ImpactReport` DTO (gates the build);
  - integration test simulates a **three-file change** over a fixture graph and
    asserts the expected `affected_symbols` + `dependent_files` (+ distances) +
    `known_tests` set, with deterministic ordering;
  - input-cap test: a >200-file input is rejected as `InvalidQuery` before any
    read; a malformed path (absolute / `..` / scheme) is rejected (CE-6);
  - degradation fixture: a warming / cold graph yields `NotReady`, never a file
    read (CE-7);
  - root-admission test: a cross-worktree `workspace_root` is rejected
    daemon-side (C3).
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs` (+ the `GctxDispatch` impl in
  `save_time.rs`), `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-011 (Merged 2026-06-16 via #2685), GCTX-010 (Merged
  2026-06-16 via #2657), GV2-011 (Released/Shipped v0.8.0-beta), GV2-026
  reverse-impact depth lever (Merged 2026-06-14 via #2594) — all closed.

---

#### GCTX-013: `anvil_affected_tests` tool

- **Status:** Merged 2026-06-17 via #2700 — built directly on the GCTX-010/011/012 spine (all Merged):
  the sealed `anvil-gctx-types` DTOs, the single `GctxProjector` choke point, the
  `GctxDispatch` RPC surface, GCTX-012's `is_test_file` heuristic, and the
  reverse-impact walk. **No new graph substrate** — it adds *attribution* and
  *coverage gaps* over the same warm-graph reads GCTX-012 already uses, plus the
  dependency graph's **forward** edges (`dependencies_of`) for the evidence link.
  Contract fixed by the GCTX-001 spec ("test files + evidence edges with explicit
  heuristic/incomplete-coverage markers, identity-only").
- **Intent:** Let assistants ask which tests are likely relevant to a change —
  and, conversely, which changed files have **no** test exercising them — so they
  can run the right tests and spot uncovered edits.
- **Expected Outcome:** `anvil_affected_tests` accepts **changed file paths only**
  (workspace-relative, ≤200, CE-6 — never diff content) and returns a
  deterministic, **identity-only** `AffectedTestsReport`, projected daemon-side
  through the single `GctxProjector` and served over a new read-only
  `anvil/gctx/affected_tests` RPC on the existing `GctxDispatch`. The report
  carries:
  - `tests` — each **test file** (recognised by GCTX-012's `is_test_file`
    heuristic) that imports a changed file within the depth bound, with its
    **evidence edges**: the changed source files it depends on
    (`dependencies_of(test) ∩ changed_set`) and the traversal distance — the
    *why* that connects the test to the change;
  - `coverage_gaps` — changed **non-test** files with **no** test importer within
    the depth bound (the "you changed X, nothing tests it" warning);
  - a `heuristic: true` marker stating relevance is **import-derived, not
    execution-verified**, file-keyed (symbol-level coverage is out of scope) —
    so an assistant never treats the result as authoritative coverage;
  - counts-only `summary` (tests, evidence edges, coverage gaps, truncation).
- **Acceptance criteria (inherit the GCTX-010/011/012 CE spine):**
  - **CE-5** — the report is a sealed `anvil-gctx-types` DTO (no `PathBuf`, no
    session-local id, no source text); the structural no-leak test covers the new
    `AffectedTestsReport` type. The MCP crate links only `anvil-gctx-types`.
  - **CE-6** — the ≤200 changed-file input cap + per-path validation reuse the
    GCTX-012 helpers; paths-only (no diff content reaches the daemon).
  - **Depth bound** — test discovery and coverage-gap detection are clamped by the
    GV2-026 `clamp_reverse_impact_depth` / `MAX_REVERSE_IMPACT_DEPTH` lever and a
    node budget; over-bound results carry truncation metadata.
  - **Heuristic honesty** — the report explicitly marks itself import-heuristic;
    a changed file with no resident test importer is surfaced as a coverage gap,
    never silently omitted.
  - **CE-7** — identity-only carve-out: a warming / cold graph yields a structured
    `NotReady` / `Unavailable` outcome (with the C1 re-warm trigger), never a
    whole-file fallback.
  - **C3** — the daemon validates the client `workspace_root` against the
    connection's admitted-root set (reuse the `SaveTimeConn` gate).
  - **CE-10 / CE-11** — enum-only telemetry and the `ANVIL_GCTX_EGRESS`
    kill-switch, reusing the GCTX-010 surfaces.
- **Validation:**
  - structural no-leak test for the new `AffectedTestsReport` DTO (gates the
    build);
  - fixture test over a source `s.ts`, a test `s.test.ts` importing it, and a
    second changed source `u.ts` with no test → asserts `s.test.ts` appears with
    an evidence edge to `s.ts`, `u.ts` is in `coverage_gaps`, and the heuristic
    marker is set;
  - input-cap + path-validation rejection (CE-6); warming-graph degradation
    (CE-7); cross-worktree `workspace_root` rejection (C3); deterministic
    ordering.
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs` (+ the `GctxDispatch` impl in
  `save_time.rs`), `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** medium — test *relevance* is an import heuristic (not
  execution-based), so the report is explicitly framed as such; the projection
  itself is mechanical on the existing spine.
- **Priority:** High
- **Dependencies:** GCTX-012 (Merged 2026-06-17 via #2693), GCTX-010/011 (Merged),
  GV2-011 (Released/Shipped v0.8.0-beta), GV2-026 reverse-impact depth lever
  (Merged 2026-06-14 via #2594) — all closed.

---

#### GCTX-014: `anvil_find_callers` symbol caller traversal

- **Status:** Merged 2026-06-17 via #2715 — the `anvil_find_callers` MCP tool +
  `anvil/gctx/find_callers` RPC projecting the GCALL-003 `callers_of` read API as
  a sealed identity-only DTO (`heuristic` per caller + report `partial`),
  reusing the GCTX-010/011 spine; CALL-1..CALL-5 met. The GCALL substrate now
  provides symbol-level call edges and a bounded `callers_of` read API with the
  CALL-1 `heuristic` marker; GCALL-007 approved the egress posture.
  Split out of GCTX-011 (2026-06-17) so the ready
  `anvil_find_dependents` half shipped independently. The call-graph substrate is
  now owned by the **[symbol-call-graph (GCALL)](symbol-call-graph.aps.md)**
  module (filed 2026-06-17). **GCALL-003** (resident call edges + `callers_of`
  read API) Merged 2026-06-17 via #2708; **GCALL-007** (caller-egress privacy
  review) APPROVE-WITH-CONDITIONS 2026-06-17 ([verdict](../reviews/2026-06-17-gcall-caller-egress-privacy-review-verdict.md);
  CALL-1..CALL-5 folded below). The **CALL-1 substrate prerequisite** is met:
  `callers_of` now carries the per-caller `heuristic` (overload fan-out) marker
  (GCALL-003 follow-up); the report-level `partial` (unresolved callers) is
  computed in this item from the daemon `all_calls` accumulator. **Merged** —
  CE-5 structural no-leak tests extend to `CallerSummary` / `FindCallersProjection`
  (CALL-2 hard gate); the CE-5 absolute-path drop and `project_callers` keyset
  pagination tests landed with the Copilot review pass. With GCTX-014 merged the
  Phase 1 tool surface (010..014) is complete.
- **Intent:** Let assistants find the call sites of a symbol — who calls this
  function — for precise blast-radius reasoning.
- **Expected Outcome:** `anvil_find_callers` returns bounded, depth-limited,
  identity-only caller results (calling symbol identity, source file, distance,
  truncation metadata) over resident symbol-level call edges, reusing the GCTX-010
  sealed-DTO + `GctxProjector` + `GctxDispatch` spine and the GCTX-011 CE gates.
- **Acceptance criteria (GCALL-007 caller-egress conditions, folded):**
  - **CALL-1 (NEW, hard)** — the result carries `heuristic` (overload-fan-out
    over-inclusion) and `partial` (unresolved / over-cap call sites) markers per
    ADR-086 §1, and the tool description states it is a best-effort static
    import-derived over-approximation, not an authoritative caller set. Requires
    the GCALL-003 substrate markers first (CALL-1 prerequisite).
  - **CALL-2 (CE-5, hard)** — sealed `anvil-gctx-types` caller DTO carrying only
    `SymbolIdentity` + `distance` (+ markers + counts-only `RedactionSummary`);
    no `PathBuf`, no session-local `u64` id, no source text; the structural
    no-leak test extends to the new response type; MCP crate links only
    `anvil-gctx-types`.
  - **CALL-3 (CE-6)** — keep the `callers_of` node budget + GV2-026 depth clamp;
    add server-minted opaque pagination + `MAX_PAGE_LIMIT` + query-param
    validation, reusing the GCTX-010/011 cursor machinery; truncation surfaced,
    never silent.
  - **CALL-4 (CE-7/10/11)** — a `FindCallersOutcome` enum shaped like
    `FindDependentsOutcome`; `telemetry_outcome() -> GctxOutcome` (PII-free labels
    only); `ANVIL_GCTX_EGRESS=0` re-read per call; no source fallback on
    warming/stale (`not_ready` / `partial`).
  - **CALL-5 (CE-1/2/3, confirmatory)** — identity-only; any call-site /
    caller-body **source-text** egress is a CE-1-gated Phase-2 escalation, out of
    scope here.
- **Validation:** Fixture tests over a call graph cover direct callers, transitive
  callers at bounded depth, recursion / cycles, and overload disambiguation; plus
  the CE-5 no-leak (CALL-2) and CE-7 degradation (CALL-4) gates and the CALL-1
  heuristic/partial markers.
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** **[GCALL-003](symbol-call-graph.aps.md)** (resident call
  edges + caller read API) + **[GCALL-007](symbol-call-graph.aps.md)**
  (caller-egress privacy review), from the symbol-call-graph (GCALL) module —
  kernel call-site extraction into `FileSymbols`, lifting `EdgeType::Calls` into
  the resident `SymbolGraph` via `apply_delta` within the ADR-031 save-time
  budget, plus the egress sign-off. GCALL is producer-side substrate (mirrors how
  GCTX consumes GV2), gated on its own design ADR (GCALL-001). Also GCTX-010
  (Merged), GCTX-011.
- **Source:** Split from GCTX-011 via planning-workflow (2026-06-17); see the
  GCTX-011 scope note.

---

### Phase 2 — Context Slicing

#### GCTX-020: Token-count estimator

- **Status:** Done 2026-06-20 — implemented as the parser-free
  `anvil_graph_cache::estimate_gctx_tokens` API, re-exported through
  `anvil_kernel::graph`, with deterministic fixed-corpus and oversized-input
  tests. This item emits counts and metadata only; snippet source text remains
  gated to GCTX-021/022/023.
- **Intent:** Provide a deterministic token estimator for source snippets and
  graph summaries.
- **Expected Outcome:** A small Rust estimator returns deterministic,
  conservative token estimates for source snippets and identity-only graph
  summaries, documents its accuracy envelope across model families, and is fast
  enough for interactive MCP planning calls. It has no provider network
  dependency, no source-text logging or telemetry, bounded input handling, and a
  stable corpus fixture that future slicer work can reuse.
- **Acceptance criteria:**
  - Determinism — identical input bytes, language hint, and estimator version
    return identical counts across runs and platforms.
  - Conservative budget posture — the estimator may over-count within the
    documented envelope, but must not under-count the fixed corpus against its
    recorded reference counts.
  - Boundary safety — this item exposes counts and metadata only; it does not
    enable snippet text egress, read arbitrary files, or bypass the GCTX-021/022
    redaction path.
  - Bounded cost — large inputs are capped or rejected with structured metadata,
    and the implementation is linear in accepted input size.
  - Fixture contract — the fixed corpus and expected counts are checked in so
    GCTX-022 can prove slices stay under budget without inventing a second
    estimator.
- **Validation:** `cargo test --workspace gctx_token_estimator` plus the fixed
  corpus unit tests compare against known counts and cover cap/rejection cases
- **Files:** `crates/anvil-graph-cache/src/tokens.rs`,
  `crates/anvil-graph-cache/src/lib.rs`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCTX-001 (Merged 2026-06-15 via #2628)

---

> **Phase 2 readiness (2026-06-23).** GCTX-021..023 promoted **Draft → Ready**.
> The snippet CE gates (PV-9 CE-1/CE-2/CE-3/CE-5/CE-6/CE-7/CE-9/CE-11/CE-12) are
> written into the item text below, and the substrate prerequisite — symbol spans
> and a per-file content hash on the resident graph — is filed as
> **[GV2-032](graph-v2-foundation.aps.md)** (the resident `SymbolNode` carries no
> span today; `graph.rs` defers span population to "a consumer", which is GCTX-021).
> The original `anvil-kernel/src/graph/{snippet,slice}.rs` framing is **superseded**:
> all source-text handling runs **daemon-side** in `anvil-gctx-egress` through the
> single CE-5 `GctxProjector`, never in the MCP/kernel client (ADR-084). The
> `gctx.egress` manifest flag (CE-9, deferred from GCTX-010 C4) lands with GCTX-021,
> the first snippet implementation file.

#### GCTX-021: Symbol snippet extractor (daemon-side, egress-gated)

- **Status:** In Progress (`feat/gctx-021-snippet-extractor`, started 2026-06-24;
  GV2-032 substrate merged via #2896; secret-scan wiring = injected redactor per
  ADR-064, not a direct `anvil-checks` dep on the leaf projector) — architecture
  settled by [ADR-084](../decisions/084-gctx-graph-handle-access.md)
  (daemon-side projection) and the snippet gates fully specified by the
  [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
  (CE-1/CE-2/CE-3/CE-5/CE-7). Substrate prerequisite **[GV2-032](graph-v2-foundation.aps.md)**
  (Ready) supplies the byte span to locate and the content hash to freshness-check;
  without it there is no producer for spans. Extraction runs **daemon-side** in
  `anvil-gctx-egress` through the single `GctxProjector`, reusing the GCTX-010
  sealed-DTO + `GctxDispatch` spine.
- **Intent:** Return the bounded source span of a graph-selected symbol as the
  single permitted source-text carrier, so assistants read one symbol body — not a
  whole file — and only when egress is explicitly enabled.
- **Expected Outcome:** For a symbol resolved by `SymbolIdentity`, the daemon reads
  **only** the bytes of its GV2-032 span from the session-pinned, admitted workspace
  root (CE-8 — never a whole-file read, never outside the graph → redaction → budget
  pipeline), runs the emitted text through the CE-2 secret scan and CE-3 path filter,
  freshness-checks the file against the recorded content hash (CE-7), and returns a
  sealed `SnippetResult { file (rel), span: ByteRange, language, text, truncated,
  omitted_bytes }` — the **only** `anvil-gctx-types` type permitted to carry source
  text (the CE-5 carve-out). With the opt-in off or the per-request capability
  unasserted, the response is identity-only (span-as-location, no `text`).
- **Acceptance criteria (snippet CE gates):**
  - **CE-1 (hard)** — text is returned only when (a) the `gctx.egress` flag is on
    **and** (b) the request asserts the per-call snippet capability; otherwise
    identity-only. Default posture is identity-only.
  - **CE-5 (hard)** — `SnippetResult` is the sole text carrier in `anvil-gctx-types`;
    the structural no-leak test is **amended** to permit the banned-name fields
    (`text`/`span`/`byte`) on exactly this type (behind CE-1) and continues to ban
    them on every other DTO. No `PathBuf`, no session-local `u64` id.
  - **CE-2** — every emitted snippet passes a deny-by-default secret scan over the
    **emitted text** (reuse the `anvil-checks` SCAN-002 4 KiB-per-line guard at
    `crates/anvil-checks/src/secret/types.rs`; a skipped line **fails closed →
    redact**); a hit redacts the span **deterministically** and bumps a counts-only
    `redaction_summary`.
  - **CE-3** — egress-side path deny-list (`.env*`, `*.pem`/`*.key`/`id_rsa*`/`*.p12`,
    `.git/**`, `secrets/`/`.aws/`/`.ssh/`/`.gnupg/`) **plus gitignored files** on the
    snippet path specifically (only the welcome scan honours `.gitignore`; all other
    scans use `standard_filters(false)`, so such files are graph-resident); a denied
    path is **omitted entirely** (not merely redacted) and counted in metadata.
  - **CE-7** — re-validate the file's current content hash against GV2-032's recorded
    hash before emitting; on mismatch / warming / stale / disabled return a structured
    outcome with the `text` field **absent** — never a `std::fs::read_to_string`
    fallback.
  - **CE-9 (FLAGCAT)** — the `gctx.egress` entry lands in **this** item (first snippet
    impl file): `class: rollout`, `valueType: boolean`, `defaultVariant: disabled`,
    `createdFor: GCTX-001`, opt-in via `ANVIL_GCTX_EGRESS=1`, with **both** a Rust
    consumer gate and a TS consumer reader (avoid the orphan-flag drift failure).
  - **CE-8 / C3** — the span read is confined to the admitted workspace root via the
    `SaveTimeConn` gate; symlink / `..` / cross-worktree paths rejected daemon-side.
- **Validation:** structural no-leak test (gates build; asserts only `SnippetResult`
  carries text); TS/JS, Rust, Python snippet-extraction fixtures; a planted-secret
  fixture asserts redaction + fail-closed on scan error (CE-2); a stale-file fixture
  (hash mismatch) asserts the `text` field is absent (CE-7); a `.env`/gitignored
  fixture asserts omission (CE-3); flag-off asserts identity-only (CE-1).
- **Files:** `crates/anvil-gctx-types/` (the `SnippetResult` DTO + amended no-leak
  test), `crates/anvil-gctx-egress/` (the daemon-side extractor through
  `GctxProjector`), `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs`, `flags/manifest.json` (the `gctx.egress`
  entry), `crates/anvil-cli/src/mcp/` (consumer gate + TS reader)
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** **[GV2-032](graph-v2-foundation.aps.md)** (span + content-hash
  producer, Ready), GCTX-010 (sealed-DTO + `GctxProjector` + `GctxDispatch` spine,
  Merged #2657), GCTX-001 (CE contract, Merged #2628)

---

#### GCTX-022: Budget-bounded context slicer

- **Status:** Ready — daemon-side, in `anvil-gctx-egress`; depends on GCTX-021 (the
  snippet carrier) and GCTX-020 (`estimate_gctx_tokens`, **Done 2026-06-20**).
- **Intent:** Turn a set of graph-selected symbols into the smallest useful set of
  snippets under a caller token budget, deterministically.
- **Expected Outcome:** A slicer that orders candidate snippets by a stable key,
  **redacts each (CE-2) before measuring it** with `estimate_gctx_tokens` (so a
  redacted span still counts honestly toward the budget), admits snippets until the
  budget is reached, and returns omitted-context metadata when it truncates. Enforces
  the CE-6 per-session snippet **byte** ceiling keyed on `(file, ByteRange)` position
  identity (never on text content), so overlapping-span calls cannot reassemble a
  whole file.
- **Acceptance criteria:**
  - **determinism** — identical symbol set + budget + graph state → byte-identical
    slice (stable ordering, no map-iteration leakage), so GCTX-022's property tests
    do not flake.
  - **budget property (CE-6)** — a property test proves the returned token estimate
    never exceeds the requested budget across randomised inputs.
  - **redact-before-budget (CE-2)** — a redacted span counts at its redacted size,
    proven by a planted-secret fixture.
  - **per-session byte ceiling (CE-6)** — accumulates on `(file, ByteRange)` identity
    independent of the per-call token budget; an overlapping-request sequence is
    capped and surfaced as `budget_exceeded`.
- **Validation:** property test (estimate ≤ budget); a determinism golden; the
  byte-ceiling + redact-before-budget fixtures.
- **Files:** `crates/anvil-gctx-egress/` (the slicer through `GctxProjector`),
  `crates/anvil-gctx-types/` (the slice / omitted-context DTO),
  `crates/anvil-graph-cache/src/tokens.rs` (the GCTX-020 `estimate_gctx_tokens`
  consumer, no second estimator)
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GCTX-020 (Done 2026-06-20), GCTX-021

---

#### GCTX-023: `anvil_symbol_context` tool

- **Status:** Ready — the headline assistant context tool, composing the Phase-1
  query spine (search + impact) with GCTX-021 snippets and GCTX-022 budgeting into
  one MCP tool on the existing `GctxDispatch`. MCP target settled by
  [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) (Rust `anvil mcp serve`).
- **Intent:** Given a symbol or file, return the bounded context an assistant needs
  to work safely — identity by default, source text only under explicit opt-in.
- **Expected Outcome:** `anvil_symbol_context` combines symbol search, local impact,
  GCTX-021 snippet extraction, and GCTX-022 budgeting into one deterministic response
  over a new read-only `anvil/gctx/symbol_context` RPC, projected daemon-side through
  the single `GctxProjector`. **Identity-only by default**; source text only when the
  `gctx.egress` flag is on **and** the request asserts the snippet capability (CE-1).
  Carries the counts-only top-level `redaction_summary` (`fields_suppressed`,
  `snippets_truncated`, `fully_suppressed_symbols`, `outcome`) (CE-11) and degrades
  via the named `GctxOutcome` (no whole-file fallback, CE-7).
- **Acceptance criteria:**
  - **CE-1 / CE-7 / CE-11** as above; the `GctxOutcome` telemetry enum gains
    `Redacted` and `BudgetExceeded` (already reserved `#[non_exhaustive]` in
    `anvil-gctx-types`), enum-only labels with no names/paths/query/snippet text
    (CE-10).
  - **CE-5** — the response is a sealed `anvil-gctx-types` DTO; source text only via
    the embedded `SnippetResult`s; the no-leak test extends to the new response type.
  - **CE-12 (consent)** — enabling snippet egress is an explicit operator action with
    the one-line consequence statement (*"source text from matched symbols, secret-
    scanned and path-filtered, will be sent to the connected assistant/LLM provider"*);
    no GCTX tool auto-enables snippets on first use. Documented in GCTX-032.
- **Validation:** MCP integration test against a fixture (identity-only with the flag
  off; text + `redaction_summary` with the flag on + capability asserted);
  determinism golden; warming-graph degradation (CE-7).
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`, `crates/anvil-intercept/src/ipc.rs`,
  `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** medium
- **Priority:** Critical
- **Dependencies:** GCTX-022, GCTX-021, GCTX-010/011/012 (the query spine, all Merged)

---

### Phase 3 — Resources, Benchmarks, Docs

#### GCTX-030: `graph://` MCP resources

- **Status:** Merged 2026-06-18 via #2772 — the three read-only `graph://`
  resources (`stats`/`symbols`/`edges`) over the daemon `anvil/gctx/*` surface,
  identity-only through the CE-5 `GctxProjector`, with CE-6 keyset pagination, a
  `bounded` honesty flag on edges, warm-on-`NotReady`, and the `resources`
  capability + `resources/list`/`resources/read` dispatch. `graph://symbols`
  reuses the GCTX-010 `search_symbols` RPC. Batch Council ran pre-PR (determinism
  BLOCK on the edge enumeration fixed: sorted file/edge walk + deterministic
  truncation) plus the Copilot follow-ups; cleanup agent advances Merged →
  Released/Shipped on the next release tag that includes #2772.
- **Intent:** Expose safe graph summaries and stats as read-only MCP resources —
  the identity-only **resource** surface, distinct from (and not dependent on) the
  Phase-2 snippet tools GCTX-020..023.
- **Expected Outcome:** `graph://symbols`, `graph://edges`, and `graph://stats`
  exist with pagination, redaction, and warming/stale metadata. Each is
  **identity-only** through the CE-5 `GctxProjector` choke point (same sealed
  posture as the GCTX-010..014 tools — no source text); `stats` carries the
  `AssuranceState` warming/stale signal; large listings page via the CE-6 keyset
  cursor scheme. The `initialize` result advertises the `resources` capability.
- **Validation:** `resources/list` returns the three URIs; `resources/read` round
  trips each with pagination + the CE-5 no-leak test battery extended to the
  resource payloads; `graph_disabled`/kill-switch (`ANVIL_GCTX_EGRESS`) and
  `NotReady`/warming states surface as for the tools.
- **Files:** `crates/anvil-cli/src/commands/mcp.rs` (add `resources/list` +
  `resources/read` arms to the JSON-RPC dispatch at the `tools/list`/`tools/call`
  match, and advertise the resources capability in `initialize`); a new
  `crates/anvil-cli/src/mcp/resources/` module mirroring `mcp/tools/`; reuse the
  daemon `anvil/gctx/*` RPC (ADR-084) + `anvil-gctx-egress` / `anvil-gctx-types`
  sealed DTOs.
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** GCTX-001, GCTX-002 (both **Merged**) — readiness-verified
  2026-06-18 against `origin/main`: deps satisfied, scope identity-only, MCP
  target settled by ADR-083 (Rust RMCPF `anvil mcp serve`) and the graph-access
  path by ADR-084. **Independent of GCTX-020..023** (the snippet line, in flight
  in a sibling worktree); shares the `mcp` dispatch + egress crates with it, so
  coordinate landing order / rebase to avoid file collisions on
  `commands/mcp.rs` and the gctx-egress DTOs.

---

#### GCTX-031: Token-reduction benchmark harness

- **Status:** Draft
- **Intent:** Measure whether graph context delivery reduces assistant context
  size versus naive file-reading baselines.
- **Expected Outcome:** Reproducible benchmark reports token-reduction ratios for
  fixed fixture repos and representative change sets.
- **Validation:** Benchmark runs in CI or documented local command and emits JSON
  plus a README table
- **Files:** `crates/anvil-bench/src/scenarios/token_reduction.rs`, `README.md`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCTX-023

---

#### GCTX-032: User guide for assistant graph context

- **Status:** Draft
- **Intent:** Document how users should wire graph-context tools into assistant
  workflows without confusing them with launch-critical RMCP validation.
- **Expected Outcome:** Guide explains supported clients, example workflows,
  redaction behaviour, stale graph states, and when to prefer RMCP validation
  over graph context.
- **Validation:** Manual walkthrough with Claude Code and Cursor after chosen MCP
  delivery target lands
- **Files:** `docs/guides/ai-context-delivery.md`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** GCTX-023, GCTX-030

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Context delivery starts defining Graph v2 requirements | Medium | High | GV2 owns schemas, IDs, deltas, persistence, and hot indexes |
| Users confuse RMCP validation with graph context tools | Medium | Medium | GCTX-032 separates launch validation from optional context delivery |
| Sensitive code or provenance leaks through MCP | Medium | High | GCTX-001 redaction rules and security review before tools are Ready |
| Token-reduction claims do not hold up | Medium | Medium | GCTX-031 makes the benchmark reproducible before claims are published |
| MCP target changes mid-work due to RMCPF | Medium | Medium | GCTX-002 records target decision before implementation starts |

## Decisions

1. **GCTX is a projection, not a foundation** — GV2 owns graph substrate work.
2. **Anvil-first alignment** — agent context is a secondary product benefit of
   the enforcement/provenance graph.
3. **Separate launch validation from graph context** — RMCP ships pre-write
   validation for the current release; GCTX ships optional graph context later.
4. **MCP target explicit** — graph context tools wait for GCTX-002 to choose TS
   interim, Rust RMCPF, or both.

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 0 — Delivery Contract | 2 | Complete (GCTX-001 Merged #2628, GCTX-002 Merged #2619) |
| 1 — Graph Query Tools | 5 | GCTX-010 Merged #2657 (pilot); GCTX-011 Merged #2685 (`find_dependents`); GCTX-012 Merged #2693 (`impact_of_change`); GCTX-013 Merged #2700 (`affected_tests`); GCTX-014 Merged #2715 (`find_callers`) |
| 2 — Context Slicing | 4 | GCTX-020 Done; GCTX-021..023 Ready (PV-9 snippet gates folded; substrate prerequisite GV2-032 filed) |
| 3 — Resources, Benchmarks, Docs | 3 | GCTX-030 Merged #2772; GCTX-031/032 Draft |
| **Total** | **14** | **9/14** |
