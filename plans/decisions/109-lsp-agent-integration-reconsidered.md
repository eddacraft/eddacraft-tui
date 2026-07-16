# ADR-109: LSP as an agent-integration surface, reconsidered

## Status

**Accepted** — 2026-07-16, operator (Josh). Resolves RTAI-005's un-park
question in principle: anvil supports LSP alongside MCP as a matter of
strategy, not a build/no-build toss-up. Scheduling remains open — see
Decision.

> Amends [ADR-083](./083-gctx-mcp-delivery-target.md) §"Alternatives
> Considered" only — narrows its "no capability negotiation" clause as
> applied to LSP. Does **not** reopen GCTX-002: the graph-context MCP
> delivery target stands unchanged. Separately, resolves RTAI-005's
> parking rationale under ADR-033.

## Date

2026-07-16

## Context

ADR-083 (Accepted 2026-06-15) chose the Rust MCP server (`anvil mcp serve`)
as the delivery target for GCTX's assistant-facing graph tools. Its
alternatives table dismissed a non-MCP channel on three grounds:

> "Loses the standard agent integration story (Claude Code, Cursor,
> Continue, Zed, etc.); creates yet another context transport that agents
> must learn; no capability negotiation or resource model"

Of those three, only the **capability-negotiation** clause is being
corrected here. LSP already has a capability-negotiation handshake
(`initialize` / `capabilities`) — the exact property ADR-083 says a non-MCP
channel lacks — so that specific clause doesn't hold for LSP. LSP was
never evaluated on its own merits in ADR-083; it was folded into "direct
non-MCP channel" and dismissed along with weaker alternatives. The other
two clauses (a second transport to build/maintain; no shared agent-tool
story across Claude Code/Cursor/Continue/Zed) are **not** addressed by
this correction and may still hold — this ADR does not claim LSP clears
the whole objection, only that one clause was never actually tested
against it. GCTX's MCP target (resource model, tool composition over
`graph://`) is unaffected either way.

Separately, RTAI-005 (`anvil lsp`, the mid-edit diagnostics LSP server) was
reframed 2026-06-02 and parked under ADR-033 with an explicit un-park
condition: *"a concrete demand signal (an editor/user asking, or a demo) —
not surface completeness."* At the time, no such signal existed. The
2026-06-03 graph-backed-navigation brainstorm records that LSP was the
operator's original direction for editor integration, predating ADR-083's
decision to route GCTX's graph-context delivery through MCP instead — so
interest in LSP here is a return to that original direction, not new scope
invented after the fact. That brainstorm's navigation/query capabilities
(find-usages, impact radius, codeLens) remain explicitly out of scope for
RTAI-005 itself, which stays diagnostics-only; see "Scope note" below.

**Two distinct categories of signal, as of 2026-07-15/16:**

1. **Market/ecosystem trend evidence**, tracked as part of the operator's
   ongoing watch on agent-tooling direction: opencode uses LSP natively for
   agent code intelligence (not just human diagnostics); Claude Code ships
   LSP plugin support; the GitHub Copilot app uses LSP for its own
   code-intelligence pipeline; and desktop-app wrappers around CLI coding
   tools are currently a fast-growing pattern, with VS Code increasingly
   used as an agent command centre rather than solely a human editor. This
   is directional context, not a per-product ask — offered as evidence the
   "agents speak MCP, editors speak LSP" split assumed implicitly by both
   ADR-083's dismissal and RTAI-005's parking no longer cleanly holds.
2. **A specific, product-level demand signal**, which is what actually
   satisfies RTAI-005's stated un-park bar: a live prospect conversation
   was **specifically about LSP** — the prospect raised LSP integration
   directly, and independently drew the ReSharper-capability comparison
   themselves in that same conversation. This is a real "editor/user
   asking," not a hypothetical, and not an internally-generated framing
   retrofitted onto external interest.

**Scope note:** the ReSharper comparison, taken at face value, spans more
than RTAI-005's scope. Most of the "strong fit" capability set (impact
analysis, find-usages, architecture conformance, structural search) is
already computed by shipped graph work (GCTX, GCALL/ADR-086) and exposed
over MCP — but only to agents that call MCP tools, not natively inside an
editor's own "Find Usages" / codeLens UI. That presentation-layer gap is
what the 2026-06-03 brainstorm's graph-backed-navigation idea targets, and
it remains a separate, larger, not-yet-designed item sequenced *after*
RTAI-005, per that brainstorm's own text. RTAI-005 itself is narrower: a
diagnostics-only, advisory-only thin frontend over the existing
`scan_buffer` RPC. The demand signal above supports reopening RTAI-005's
un-park question; it does not by itself justify building the
graph-backed-navigation layer, which needs its own scoping pass (and
likely its own ADR, given its ADR-063 hot-path-boundary implications) if
and when it's taken up.

## Decision

**1. The ADR-083 correction stands.** Its "no capability negotiation"
clause does not generalize to LSP specifically — the two other stated
objections to non-MCP channels are unaffected and untested here. GCTX-002's
MCP delivery target is unaffected.

**2. Protocol plurality, not a single consistent agent story.** MCP and
LSP are both industry standards. anvil's integration strategy is to meet
people and agents where they already work, not to consolidate on one
protocol for architectural tidiness. RTAI-005 is un-parked **in
principle**: anvil supports LSP alongside MCP as a parallel,
standards-based integration surface for the mid-edit diagnostics use
case — this is not framed as MCP-vs-LSP, and it does not reopen GCTX-002's
choice of MCP for graph-context delivery, which is a separate surface with
a separate rationale (resource model, tool composition).

**What remains open:** scheduling and scope, not direction. This ADR
authorizes building LSP support alongside MCP; it does not itself order a
build. Sizing the RTAI-001-style throwaway spike, sequencing it against
current active work, and re-confirming RTAI-005's thin-frontend/advisory-
only scope before work starts are separate steps the operator still owns
(see Consequences).

## Rationale

The capability-negotiation correction stands on its own regardless of the
demand-signal question: ADR-083's own text supports it directly (see
Context). The protocol-plurality decision follows from reading the two
signal categories together: the ecosystem trend (opencode, Claude Code,
Copilot app all treating LSP as an agent-facing surface, not just an
editor one) and the specific product signal (a prospect asking about LSP
directly) don't argue for picking MCP over LSP or vice versa — they argue
against needing to pick at all. MCP and LSP serve different, non-competing
audiences: MCP for agents that call tools directly (GCTX, RMCPF), LSP for
the much larger set of editors and agent-in-editor sessions that already
speak it natively. The cost of supporting both is one additional thin
protocol surface — RTAI-005 is explicitly scoped as a thin frontend over
the already-shipped `scan_buffer` RPC, not a second validation engine —
which is why "support both" is affordable rather than a maintenance
doubling.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Support both MCP and LSP (chosen)** | Meets people/agents where they already work; each protocol serves an audience the other doesn't reach natively; RTAI-005 is a thin add-on, not a second engine | One more protocol surface to maintain, however thin |
| Consolidate on MCP only | One integration story, less surface area | Leaves the much larger LSP-native editor/agent audience (opencode, Claude Code plugin, Copilot app, any `lspconfig`/`eglot` client) unreached without a bespoke per-editor extension |
| Consolidate on LSP only | Would unify around the operator's original direction | Abandons the shipped GCTX/RMCPF MCP investment and the tool-composition/resource-model capabilities LSP doesn't have — not on the table |
| Zero-build: demo existing MCP tools in an agent-in-editor session | No new surface | Doesn't reach non-agent, non-MCP-wired editors at all; answers "can it," not "does it show up where people work" |

Scheduling (when the spike happens, how it's sized) is still deferred to a
separate step — see Consequences.

## Consequences

- **Positive:**
  - Corrects the record narrowly — future references to ADR-083 as having
    evaluated and rejected LSP on its own merits should cite this ADR's
    correction to the capability-negotiation clause specifically, not
    treat the whole non-MCP dismissal as settled either way.
  - Settles the MCP-vs-LSP framing question: anvil is not choosing one
    integration story. Future work (this RTAI-005 diagnostics surface, and
    potentially the graph-backed-navigation idea later) can build LSP
    surfaces without re-litigating whether that contradicts the GCTX/MCP
    investment.
- **Negative:** None yet; scheduling/spike sizing not yet decided.
- **Risks:**
  - If RTAI-005 is un-parked without re-litigating scope, it could
    re-absorb the DRVR-002/DRVR-003 editor-driver dependencies it was
    explicitly decoupled from in the 2026-06-02 reframe.
  - A build decision must not implicitly green-light the
    graph-backed-navigation brainstorm or its ADR-063 hot-path-boundary
    implications under the RTAI-005 label — that remains a separate,
    unscoped item.
  - Any spike or build work competes for capacity against the current
    active `In Progress` set (DASH, GCTX-021 promotion, MCPX rollout,
    JOURNEY follow-ups, ACTTUI, MLP2, CIB backlog per `plans/index.aps.md`)
    — this ADR does not authorize scheduling it ahead of that work.
- **Mitigations:** Any build decision should re-confirm the "thin
  frontend over `scan_buffer`, advisory-only" framing from RTAI-005 before
  scope grows, and should explicitly re-scope rather than inherit the
  graph-backed-navigation brainstorm's larger surface.

## References

- [ADR-083](./083-gctx-mcp-delivery-target.md) — GCTX MCP delivery target;
  carries a reciprocal amendment note pointing back here
- [ADR-033](./033-park-ide-mcp-retire-ts-scanner.md) — parks RTAI-005 /
  DRVR-003
- [`realtime-ai-validation.aps.md`](../modules/realtime-ai-validation.aps.md)
  — RTAI-005 work item; carries a backlink to this ADR
- [`2026-06-03-anvil-lsp-graph-backed-navigation.md`](../brainstorms/2026-06-03-anvil-lsp-graph-backed-navigation.md)
  — related future idea; explicitly out of scope for both this ADR and
  the RTAI-005 un-park question
