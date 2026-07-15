# ADR-108: LSP as an agent-integration surface, reconsidered

## Status

Draft

> Amends [ADR-083](./083-gctx-mcp-delivery-target.md). Does **not**
> reopen GCTX-002 — the graph-context MCP target stands. This ADR narrows
> ADR-083's alternatives analysis and re-examines RTAI-005's parking
> rationale in light of it.

## Date

2026-07-16

## Context

ADR-083 (Accepted 2026-06-15) chose the Rust MCP server (`anvil mcp serve`)
as the delivery target for GCTX's assistant-facing graph tools. Its
alternatives table dismissed a non-MCP channel with:

> "Loses the standard agent integration story (Claude Code, Cursor,
> Continue, Zed, etc.); creates yet another context transport that agents
> must learn; no capability negotiation or resource model"

That line conflates two different things: a bespoke socket/file channel
(genuinely transport-naive, no negotiation) and **LSP**, which already has
a capability-negotiation handshake (`initialize` / `capabilities`) — the
exact property ADR-083 says a non-MCP channel lacks. LSP was never
separately evaluated in ADR-083; it was folded into "direct non-MCP
channel" and dismissed on grounds that don't hold for it specifically.

Separately, RTAI-005 (`anvil lsp`, the mid-edit diagnostics LSP server)
was reframed 2026-06-02 and parked under ADR-033 with an explicit
un-park condition: *"a concrete demand signal (an editor/user asking, or
a demo) — not surface completeness."* At the time, no such signal existed.

Since then, the agent-tooling landscape has shifted in a way that looks
like that signal:

- **opencode** uses LSP natively for agent code intelligence (not just
  human diagnostics) — LSP is already an agent-facing transport there,
  not solely an editor one.
- **Claude Code ships LSP plugin support** — LSP is a first-class way
  Claude Code itself gets code-intelligence context.
- **VS Code is increasingly used as an agent command centre**, not just
  a human editor — the audience for `publishDiagnostics` now includes
  agents running inside VS Code, not only the person typing.
- **The GitHub Copilot app uses LSP** for its own code-intelligence
  pipeline.

In other words: the premise "agents speak MCP, editors speak LSP" — the
implicit split underlying both ADR-083's dismissal of non-MCP channels
and RTAI-005's parking — no longer cleanly holds. Multiple agent
surfaces now consume LSP directly.

**Concrete demand signal (2026-07-15/16):** a prospective user asked,
unprompted, whether anvil offered ReSharper-like capabilities. The
resulting capability mapping showed most of the "strong fit" set (impact
analysis, find-usages, architecture conformance, structural search) is
already computed by shipped graph work (GCTX, GCALL/ADR-086) and exposed
over MCP — but only to agents that call MCP tools, not natively inside an
editor's own "Find Usages" / inline codeLens UI. That gap is exactly the
presentation layer LSP provides and MCP does not. This is the kind of
signal RTAI-005 named as its un-park condition ("an editor/user asking,
or a demo") — not surface completeness, and not a hypothetical.

## Decision

Not yet made. This addendum records the observation and reopens RTAI-005
for a build/no-build discussion; it does not itself commit to building
`anvil lsp`.

What this addendum *does* settle: ADR-083's "no capability negotiation"
argument does not generalize to LSP, so it should not be cited as a
reason LSP was considered and rejected for *any* future surface — it
wasn't considered on its own merits, only bundled with weaker
alternatives.

## Rationale

N/A — no decision yet. See Context for the reasoning that motivates
reopening the question.

### Alternatives Considered

Deferred to the RTAI-005 build/no-build discussion.

## Consequences

- **Positive:** Corrects the record — future ADRs citing ADR-083 for
  "why not LSP" should cite this addendum instead, since ADR-083 didn't
  actually evaluate LSP.
- **Negative:** None yet; no implementation decision made.
- **Risks:** If RTAI-005 is un-parked without re-litigating scope, it
  could re-absorb the DRVR-002/DRVR-003 editor-driver dependencies it
  was explicitly decoupled from in the 2026-06-02 reframe.
- **Mitigations:** Any build decision should re-confirm the "thin
  frontend over `scan_buffer`, advisory-only" framing from RTAI-005
  before scope grows.

## References

- [ADR-083](./083-gctx-mcp-delivery-target.md) — GCTX MCP delivery target
  (unaffected by this addendum)
- [ADR-033](./033-park-ide-mcp-retire-ts-scanner.md) — parks RTAI-005 /
  DRVR-003
- [`realtime-ai-validation.aps.md`](../modules/realtime-ai-validation.aps.md)
  — RTAI-005 work item and 2026-06-02 reframe rationale
- [`2026-06-03-anvil-lsp-graph-backed-navigation.md`](../brainstorms/2026-06-03-anvil-lsp-graph-backed-navigation.md)
