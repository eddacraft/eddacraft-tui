# Graph Context Delivery

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| GCTX | —     | In Progress | 3/14 |

**Last reviewed:** 2026-06-15 (Phase 0 — Delivery Contract — complete. **GCTX-001 (projection contract) Merged 2026-06-15 via #2628** — the spec [`graph-context-delivery-spec.md`](../../docs/architecture/graph-context-delivery-spec.md) folds the context-egress privacy review (PV-9) conditions CE-1..CE-12 onto the GV2-023 consumer query contract. **GCTX-002 (MCP delivery target) Merged 2026-06-15 via #2619** — discharged by [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted** (Rust RMCPF `anvil mcp serve` surface); RMCPF defers GCTX work by design, so no edit to rust-mcp-full-port. Module **In Progress, 3/14** (GCTX-010 pilot Merged 2026-06-16 via #2657; GCTX-011 `find_dependents` promoted to Ready 2026-06-17). The remaining Phase 1 tool items (GCTX-012/013, 021..023, 030) stay Draft, and GCTX-014 `find_callers` is Blocked on GV2 symbol-call-edge support (split from GCTX-011 2026-06-17); all build on the CE-5 sealed egress DTO + `GctxProjector` + structural no-leak spine that GCTX-010 established, using the daemon-RPC graph-handle path settled by ADR-084.)

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

- **Status:** Ready — builds directly on the GCTX-010 spine (Merged 2026-06-16
  via #2657): the sealed `anvil-gctx-types` DTOs, the single `GctxProjector`
  choke point in `anvil-gctx-egress`, and the `GctxDispatch` RPC surface. Scoped
  to **dependents only**; symbol-level *caller* traversal needs call edges the
  warm graph does not carry and is split out to **GCTX-014** (Blocked on GV2
  call-edge support).
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

- **Status:** Draft
- **Intent:** Given changed files or proposed edits, return affected symbols,
  dependent files, and known tests as a structured assistant-readable report.
- **Expected Outcome:** Tool accepts explicit files and optionally current git
  diff/staged state; returns deterministic `ImpactReport` JSON.
- **Validation:** Integration test simulates a three-file change and checks the
  expected affected set
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-011

---

#### GCTX-013: `anvil_affected_tests` tool

- **Status:** Draft
- **Intent:** Let assistants ask which tests are likely relevant to a change.
- **Expected Outcome:** Tool returns test files and evidence edges, clearly
  marking heuristic or incomplete coverage.
- **Validation:** Fixture test shows known source/test import links and missing
  coverage warnings
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCTX-012, GV2-011

---

#### GCTX-014: `anvil_find_callers` symbol caller traversal

- **Status:** Blocked — the warm graph carries **no symbol-level call edges**.
  The `EdgeType::Calls` / `References` variants exist in `anvil-kernel-types`, but
  the kernel symbol extractor never emits them and `FileSymbols` (the `apply_delta`
  feed) carries only `symbols` / `imports` / `reexports`, so a true call graph
  cannot be projected today. Split out of GCTX-011 (2026-06-17) so the ready
  `anvil_find_dependents` half ships independently.
- **Intent:** Let assistants find the call sites of a symbol — who calls this
  function — for precise blast-radius reasoning.
- **Expected Outcome:** `anvil_find_callers` returns bounded, depth-limited,
  identity-only caller results (calling symbol identity, source file, distance,
  truncation metadata) over resident symbol-level call edges, reusing the GCTX-010
  sealed-DTO + `GctxProjector` + `GctxDispatch` spine and the GCTX-011 CE gates.
- **Validation:** Fixture tests over a call graph cover direct callers, transitive
  callers at bounded depth, recursion / cycles, and overload disambiguation; plus
  the CE-5 no-leak and CE-7 degradation gates.
- **Files:** `crates/anvil-gctx-types/`, `crates/anvil-gctx-egress/`,
  `crates/anvil-intercept-proto/src/protocol.rs`,
  `crates/anvil-intercept/src/ipc.rs`, `crates/anvil-cli/src/mcp/tools/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** **GV2 symbol-level call-edge support (not yet filed)** —
  kernel parser call-site extraction (TS / JS / Rust) into `FileSymbols`, lifting
  `EdgeType::Calls` into the resident `SymbolGraph` via `apply_delta` within the
  ADR-031 save-time budget; a GV2-substrate addition (the GV2-023 symbol-granular
  freeze-target) that wants its own GV2 item + ADR before GCTX-014 can flip to
  Ready. Also GCTX-010 (Merged), GCTX-011.
- **Source:** Split from GCTX-011 via planning-workflow (2026-06-17); see the
  GCTX-011 scope note.

---

### Phase 2 — Context Slicing

#### GCTX-020: Token-count estimator

- **Status:** Draft
- **Intent:** Provide a deterministic token estimator for source snippets and
  graph summaries.
- **Expected Outcome:** Estimator documents its accuracy envelope across model
  families and is fast enough for interactive MCP calls.
- **Validation:** Unit tests compare against a fixed corpus of known counts
- **Files:** `crates/anvil-kernel/src/graph/tokens.rs` or Rust MCP target
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** GCTX-001

---

#### GCTX-021: Symbol snippet extractor

- **Status:** Draft
- **Intent:** Return source spans for symbols selected by graph queries without
  making assistants read entire files.
- **Expected Outcome:** Extractor returns file, start/end line, language, text,
  and truncation metadata for supported symbols.
- **Validation:** Fixture tests cover TS/JS and at least one future language once
  GV2 exposes source spans
- **Files:** `crates/anvil-kernel/src/graph/snippet.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GV2-010

---

#### GCTX-022: Budget-bounded context slicer

- **Status:** Draft
- **Intent:** Convert graph query results into the smallest useful code context
  an assistant should read first.
- **Expected Outcome:** Slicer returns deterministic snippets under the requested
  token budget with omitted-context metadata when truncated.
- **Validation:** Property test proves returned token estimate never exceeds the
  budget
- **Files:** `crates/anvil-kernel/src/graph/slice.rs`
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-020, GCTX-021

---

#### GCTX-023: `anvil_symbol_context` tool

- **Status:** Draft
- **Intent:** Provide the headline assistant context tool: given a symbol or file,
  return the bounded context slice needed to work safely.
- **Expected Outcome:** Tool combines search, impact, snippet extraction, and
  token budgeting into one response with deterministic ordering.
- **Validation:** MCP integration test and manual smoke test against a fixture
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-022

---

### Phase 3 — Resources, Benchmarks, Docs

#### GCTX-030: `graph://` MCP resources

- **Status:** Draft
- **Intent:** Expose safe graph summaries and stats as read-only MCP resources.
- **Expected Outcome:** `graph://symbols`, `graph://edges`, and `graph://stats`
  exist with pagination, redaction, and warming/stale metadata.
- **Validation:** MCP resource listing and read tests pass
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** GCTX-001, GCTX-002

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
| 1 — Graph Query Tools | 5 | GCTX-010 Merged #2657 (pilot); GCTX-011 Ready (`find_dependents`); GCTX-012/013 Draft; GCTX-014 Blocked (`find_callers` — GV2 call-edges) |
| 2 — Context Slicing | 4 | Draft |
| 3 — Resources, Benchmarks, Docs | 3 | Draft |
| **Total** | **14** | **3/14** |
