# ADR-083: GCTX-002 — MCP target for assistant graph context delivery

## Status

**Accepted** — 2026-06-15, Josh. The two ADR-075 entry gates for the
assistant-facing surface are now both landed: this decision (GCTX-002 — the MCP
delivery target) and the [context-egress privacy review (PV-9)](../reviews/2026-06-15-gctx-context-egress-privacy-review-verdict.md)
(APPROVE-WITH-CONDITIONS, 4/4). Records that GCTX context delivery rides the Rust
`anvil mcp serve` (RMCPF) surface; the egress conditions CE-1..CE-12 from the PV-9
review fold into GCTX-001.

> **Amended (2026-07-16) by [ADR-109](./109-lsp-agent-integration-reconsidered.md).**
> The Alternatives Considered table's "no capability negotiation or
> resource model" line, as applied to a rejected non-MCP channel, is
> narrowed: LSP specifically does have a capability-negotiation handshake,
> so that clause doesn't hold for LSP on its own. The other two stated
> objections (a second agent-integration transport; no shared tool story
> across Claude Code/Cursor/Continue/Zed) are untouched. GCTX-002's MCP
> delivery-target decision itself is unaffected.

## Date

2026-06-15

## Context

ADR-075 ("Back v0.8.0-beta with the Graph V2 A′ slice (GCTX → v0.9)", Accepted 2026-06-08 via full council) deferred the assistant-facing graph product (`graph-context-delivery` / GCTX module, 0/13) and the non-critical-path GV2 consumer items (GV2-020 registry, GV2-023 query contract) to the `v0.9.0-beta` window ("The Assistant-Facing Graph").

The council explicitly flagged two entry gates for that surface:
- An unresolved *architectural* prerequisite: **GCTX-002** — "which MCP target" the context delivery lands on.
- A distinct **context-egress privacy review** (PV-9) for any export/sync/transmit of graph-derived assistant context (the 2026-06-08 GV2 privacy verdict covered only the machine-local persistence snapshot; PV-9 reserves export surfaces for a separate review).

The GCTX module (see [`graph-context-delivery.aps.md`](../archive/modules/graph-context-delivery.aps.md)) scopes assistant-facing query tools (`anvil_search_symbols`, `anvil_find_callers`, `anvil_impact_of_change`, `anvil_affected_tests`, etc.), MCP resources (`graph://*`), and context-slicing / token-reduction utilities over the GV2 query contract. It is deliberately framed as a *projection* — Graph v2 remains Anvil-first for enforcement/provenance/trust; assistant use is secondary and must not distort the substrate.

Per ADR-033 (Park IDE/MCP Surfaces; Retire TS Scanner Now, Proposed), the original TypeScript MCP server lives in `archive/anvil-mcp-server/` (frozen reference). The Rust full-port effort (`rust-mcp-full-port` / RMCPF module, currently 6/10) owns the `anvil mcp serve --stdio` path, the tool/resource/prompt capability surface, and the driver integration. RMCPF already exposes a small set of tools and has the registry + composition story for daemon-backed `anvil_check` / `anvil_gate`.

GCTX-002 must be resolved before any GCTX implementation work begins (this PR updates the item to Proposed in the module) and before the GCTX + gated GV2-020/023 items can be promoted to Ready.

## Decision

**GCTX context delivery (tools, resources, slicing) targets the Rust MCP server surface (`anvil mcp serve`) owned by RMCPF as its primary and long-term delivery mechanism.**

- New GCTX MCP tools and `graph://` resources are registered additively through the existing RMCPF capability advertisement and handler registry.
- The implementation consumes the (already GV2-backed) daemon `validate_paths` / hot-read query path where possible; it does not re-introduce a separate TS analysis surface.
- A narrow, time-bounded interim shim on any remaining TS MCP path is permitted only for migration smoke-testing and is not a supported long-term target; it must not block Rust-native progress.
- The decision is recorded here so that GCTX-001 (inventory), the concrete tool implementations, and the egress privacy review (PV-9) can all cite a stable target.

This decision does not change the existing non-graph MCP tools or the RMCPF parity contract for `check`/`gate`/`status`.

## Rationale

The TS MCP server is archived (ADR-033). Resurrecting it for GCTX would contradict the Rust engine direction (ADR-012, RCLI, RMCPF) and the "one runtime for the daemon + MCP" posture that the v0.8 save-time daemon work established. RMCPF is already the vehicle for adding new Rust-side MCP surface (registry, composition with driver/daemon paths, stdio + named-pipe parity). Landing graph context on that surface keeps the assistant projection on the same trusted, hot-path-capable, Rust-owned substrate that enforcement uses, avoiding a second analysis stack and simplifying redaction / trust boundary reasoning.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| Rust RMCPF primary (chosen) | Consistent with Rust engine + RMCPF investment; single analysis surface; reuses daemon hot-read + existing registry/composition; simplifies ADR-033 retirement story; one place for egress redaction and capability gating | Requires RMCPF to be far enough along for additive tool registration (it is — 6/10 and already exposing tools) |
| TS MCP (interim or primary) | Reuses whatever is left of the archived server for quick demos | Contradicts ADR-033 parking; resurrects a retired TS analysis path; splits the graph surface from the Rust daemon users will run by default; maintenance and redaction debt |
| Dual / shim forever | Maximum client compatibility during transition | Doubles the surface that must be kept in sync, redacted, and governed; violates "new edges only" and the Rust direction; high long-term cost for a secondary projection |
| Direct non-MCP channel (e.g. custom socket, file-based) | Avoids MCP contract surface | Loses the standard agent integration story (Claude Code, Cursor, Continue, Zed, etc.); creates yet another context transport that agents must learn; no capability negotiation or resource model |

## Consequences

- **Positive:** GCTX work can now proceed against a single, owned MCP target (RMCPF). The decision unblocks promotion of GCTX-002 and the dependent GCTX items, plus the gated GV2-020/023 consumer surface. Keeps the "Graph v2 is Anvil-first" framing intact while still delivering the v0.9 assistant product.
- **Negative:** Any GCTX timeline is now coupled to RMCPF progress for the registry/tool addition points (already partially present). Clients that only speak the old TS MCP contract will need the Rust `anvil mcp` path (or a thin launcher shim) once the TS server is fully retired.
- **Risks:** RMCPF scope drift or schedule slip delays GCTX; the chosen MCP target turns out to have an unsuitable capability or transport model for large graph context payloads.
- **Mitigations:** GCTX tools are explicitly additive and scoped (symbol search, impact, affected tests, bounded context slices); token-budget and slicing work (GCTX-00x) happens on the GV2 side before crossing the MCP boundary. Egress privacy review (PV-9) is a hard cut prerequisite and will re-examine payload shape, redaction, and transmission boundaries against the chosen target. GCTX-002 is the gate — no implementation work on the other 12 GCTX items begins until it (and the privacy review) are landed.

## References

- ADR-075 (GCTX → v0.9 deferral + explicit callout of GCTX-002 as architectural prerequisite)
- ADR-033 (park IDE/MCP surfaces; retire TS scanner + TS MCP server)
- [`graph-context-delivery.aps.md`](../archive/modules/graph-context-delivery.aps.md) (GCTX-002 work item and module constraints)
- [`rust-mcp-full-port.aps.md`](../modules/rust-mcp-full-port.aps.md) (RMCPF current state and ownership of `anvil mcp serve`)
- GV2 spine spec and hot/non-hot boundary (ADR-063)
- 2026-06-08 GV2 privacy review verdict (PV-9 explicitly reserves export/egress surfaces)
- RELEASE-PLAN.md (v0.9.0-beta phase plan — Entry decisions gate)
- `plans/index.aps.md` (NBI rank 2: GCTX entry decisions)