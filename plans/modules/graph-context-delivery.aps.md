# Graph Context Delivery

| ID   | Owner | Status | Progress |
| ---- | ----- | ------ | -------- |
| GCTX | —     | Ready  | 0/13     |

**Last reviewed:** 2026-06-15 (both ADR-075 entry gates landed — [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted** (GCTX-002 → Ready) and the [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md) filed (APPROVE-WITH-CONDITIONS, 4/4). Module promoted **Draft → Ready, 0/13**: execution is authorised. **GCTX-001 (contract) flipped Draft → Ready 2026-06-15** — its sole dep GV2-023 is Merged (#2621) and both entry gates landed; GCTX-002 already Ready. GCTX-003..013 stay Draft pending the GCTX-001 contract that folds the egress conditions CE-1..CE-12.)

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

- **Status:** Ready — sole dependency GV2-023 **Merged 2026-06-15 via #2621**, and
  both ADR-075 entry gates landed ([ADR-083](../decisions/083-gctx-mcp-delivery-target.md)
  Accepted + [PV-9 egress review](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
  filed). This is the contract item that folds CE-1..CE-12 into the spec; its
  CE-1 / CE-5 hard gates carry into the downstream Phase-1 snippet items (which
  stay Draft until that text lands).
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

- **Status:** Ready — [ADR-083](../decisions/083-gctx-mcp-delivery-target.md) **Accepted 2026-06-15** (Josh): primary target is the Rust RMCPF `anvil mcp serve` surface per RMCPF + ADR-033 parking of TS MCP; additive registration of GCTX tools/resources. Both ADR-075 entry gates are now landed (this decision + the [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)), so the item is execution-authorised. Acceptance criteria CE-8 (session-pinned root; stdio-only — a networked RMCPF transport needs a new egress review before GCTX registers there) carry into implementation.
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

#### GCTX-010: `anvil_search_symbols` tool

- **Status:** Draft
- **Intent:** Let assistants find symbols by name, kind, file, language, and
  visibility using GV2's semantic graph projection.
- **Expected Outcome:** Tool returns paginated, deterministic symbol summaries
  with source locations and redacted metadata.
- **Validation:** Integration test queries a fixture and asserts stable ordering
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-001, GCTX-002, GV2-020

---

#### GCTX-011: Caller and dependent traversal tools

- **Status:** Draft
- **Intent:** Let assistants inspect local blast radius without expensive or
  ambiguous whole-repo rereads.
- **Expected Outcome:** `anvil_find_callers` and `anvil_find_dependents` return
  bounded traversal results with distance, source file, symbol summary, and
  truncation metadata.
- **Validation:** Fixture tests cover chain, diamond, cycle, and max-depth cases
- **Files:** MCP server target decided by GCTX-002
- **Confidence:** high
- **Priority:** Critical
- **Dependencies:** GCTX-010, GV2-011

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
| 0 — Delivery Contract | 2 | Draft |
| 1 — Graph Query Tools | 4 | Draft |
| 2 — Context Slicing | 4 | Draft |
| 3 — Resources, Benchmarks, Docs | 3 | Draft |
| **Total** | **13** | **0/13** |
