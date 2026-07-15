# ADR-110: LSP as an agent-integration surface, reconsidered

## Status

Draft — **Owner: operator (Josh).** Follow-up build/no-build discussion on
RTAI-005 is owned directly by the operator; no separate venue or council is
required to close it out of Draft.

> Amends [ADR-083](./083-gctx-mcp-delivery-target.md) §"Alternatives
> Considered" only — narrows its "no capability negotiation" clause as
> applied to LSP. Does **not** reopen GCTX-002: the graph-context MCP
> delivery target stands unchanged. Separately, re-examines RTAI-005's
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

**What this ADR settles:** ADR-083's "no capability negotiation" clause
does not generalize to LSP specifically — the two other stated objections
to non-MCP channels are unaffected and untested here. GCTX-002's MCP
delivery target is unaffected.

**What remains open:** RTAI-005's un-park question. This ADR reopens it
given the LSP-specific demand signal above, but does not itself decide to
build `anvil lsp`. The operator owns that call directly — no scheduling
implication follows from this ADR alone; sizing and sequencing against
current active work (see Consequences) is a separate step.

## Rationale

The capability-negotiation correction stands on its own regardless of the
demand-signal question: ADR-083's own text supports it directly (see
Context). The RTAI-005 reopening is not itself a decision, so it has no
independent rationale beyond the two demand-signal categories above —
that discussion, and its own alternatives, belong to the operator's
build/no-build call, not to this record.

### Alternatives Considered

Deferred to the RTAI-005 build/no-build call. Note for that discussion: a
zero-build alternative exists for the presentation-layer gap named in the
Scope note — demonstrating the existing MCP-based GCTX tools running
inside an agent-in-editor session — and should be weighed against building
a second protocol surface before committing to `anvil lsp` work.

## Consequences

- **Positive:** Corrects the record narrowly — future references to
  ADR-083 as having evaluated and rejected LSP on its own merits should
  cite this ADR's correction to the capability-negotiation clause
  specifically, not treat the whole non-MCP dismissal as settled either
  way.
- **Negative:** None yet; no implementation decision made.
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
