# Anvil-as-LSP: graph-backed navigation beyond diagnostics

**Date:** 2026-06-03
**Status:** Brainstorm / future idea — captured from a design conversation
building on the RTAI-005 reframe (generic LSP server surface) and its
readiness note. Net-new design territory; needs its own work item and
probably an ADR before any build.
**Purpose:** Record the idea that once the warm graph (GV2) is resident in
the daemon, the `anvil lsp` server can evolve from a diagnostics-only
linter into a **query surface over Anvil's governance graph** — surfacing
navigation and "code shortcuts" that a normal language server does not
provide. Don't lose the framing; pick it up when GV2 + the basic LSP
server exist.

---

## Origin

RTAI-005 was reframed (2026-06-02) from a VS Code extension to a generic
`anvil lsp` language-server surface — a thin frontend over the daemon's
`scan_buffer` RPC that pushes mid-edit validation findings as
`textDocument/publishDiagnostics` to any LSP client. That is
**diagnostics-only and advisory**.

The forward-looking question: once Anvil has **hot graphs** (GV2 — the
resident warm `SymbolGraph` + `DependencyGraph` in the daemon, per
[ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md)), the
same LSP server is already a daemon client holding that graph. Can it
answer **interaction / navigation / linking** requests, not just push
diagnostics? i.e. use the LSP surface for graph-backed code shortcuts.

## The core idea

The LSP server becomes a **query surface over the governance graph**. The
diagnostics pipe and the query surface share one process, one daemon
connection, and one resident graph — so the navigation layer is largely
reuse once GV2 lands, not a second engine.

## What the warm graph could plausibly power (LSP capability map)

- **`workspace/symbol` + `textDocument/documentSymbol`** — the
  `SymbolGraph` already holds symbols (fn / class / mod / export) and
  kinds; surfacing them is a resident lookup.
- **`textDocument/references` / dependents** — 1-hop `dependents_of()` is
  hot-path-admissible per ADR-063, so "who depends on this" is cheap at
  module/symbol granularity. Full transitive references are a background
  query, not per-keystroke.
- **`textDocument/definition`** — coarse (module/symbol) navigation only;
  see the boundary below.
- **`textDocument/codeLens`** — inline annotations like "N dependents" or
  "crosses boundary X" on a symbol. Very natural fit for a governance
  graph.
- **`textDocument/codeAction`** — quick-fixes (e.g. `@anvil-ignore`
  suppression insertion), or a custom "show impact radius" action.
- **Diagnostic `relatedInformation` / `textDocument/documentLink`** — turn
  a boundary-violation edge into a clickable link to *both* endpoints.
  This is the "link via LSP" payoff: governance graph edges become
  navigable.

## The honest boundary — complement, don't compete

Anvil's graph is **governance-shaped, not type-aware**. It is built for
dependency / trust / architecture analysis, not name resolution, type
inference, generics, or macro expansion. So:

- Re-implementing precise go-to-definition is **low value** —
  rust-analyzer / tsserver do it better and it is their job. Anvil-as-LSP
  runs **alongside** the real language server, not instead of it.
- The differentiated win is the navigation those servers do **not**
  provide:
  - "Show me what breaks if I change this" (reverse-impact radius).
  - "What trust level / layer is this symbol, and does this edge cross a
    boundary?"
  - "Jump to the architectural rule this edge violates."

That governance navigation is the reason to expose the graph over LSP at
all.

## Design constraint: two timing classes

[ADR-063](../decisions/063-gv2-hot-path-boundary.md) governs the hot-path
read boundary. The key realisation is that the LSP surface has **two
distinct timing classes**, and they get different budgets:

- **Per-keystroke** (`didChange` → diagnostics): tight ADR-031 mid-edit
  budget — resident / O(1) / 1-hop reverse-impact only.
- **On-demand** (`definition` / `references` / `codeAction` / `codeLens`,
  user-initiated, occasional): a much looser budget — so these can afford
  richer-than-1-hop queries that are *banned* on the keystroke path.

Keeping these two classes separate is what makes graph-backed shortcuts
viable without violating the hot-path discipline. (Any such query must
still respect the ADR-063 stale → fallback contract — a warm miss returns
`stale`, never escalates on the hot path.)

## Sequencing

This stacks cleanly and must not jump the queue:

1. **GV2 — hot graphs** (current active investment): resident
   `SymbolGraph` / `DependencyGraph` + incremental apply-delta + 1-hop
   `dependents_of` (ADR-063 / ADR-064).
2. **RTAI-005 — basic LSP server** (diagnostics-only): `anvil lsp`
   pushes mid-edit findings; parked under ADR-033 pending a demand signal.
3. **Graph-backed LSP query layer** (this note): the navigation /
   linking / code-action surface on top of (1) and (2).

## What needs deciding (open questions)

- **Does the resident `SymbolGraph` carry source ranges precise enough**
  for `definition` / `documentSymbol`, or only coarse symbol identity?
  (Governance analysis may not need exact ranges; navigation does.)
- **Which capabilities are worth it** given the "complement, not compete"
  boundary — likely codeLens (dependents / boundary), references at
  module granularity, impact-radius code action, and edge-linking;
  probably *not* definition/hover.
- **Connection lifecycle** — inherits the RTAI-005 readiness-note question
  (per-call connect vs. a persistent daemon connection); on-demand queries
  are more forgiving than per-keystroke, but a long-lived server still
  wants a stable connection.
- **Does this widen the daemon's hot-read boundary** enough to warrant a
  new ADR amending/extending ADR-063? (Probably yes — the on-demand class
  is a new budget tier.)

## Pointers

- [`realtime-ai-validation`](../modules/realtime-ai-validation.aps.md) —
  RTAI-005 (LSP server surface) + its readiness note.
- [ADR-030](../decisions/030-surface-drivers-supersede-napi-cutover.md) —
  drivers-on-daemon architecture.
- [ADR-031](../decisions/031-validation-latency-rubric.md) — latency rubric
  (mid-edit vs interactive budgets).
- [ADR-063](../decisions/063-gv2-hot-path-boundary.md) — GV2 hot-path read
  boundary.
- [ADR-064](../decisions/064-intercept-graph-cache-crate-boundary.md) —
  `eddacraft-anvil-graph-cache` crate (`SymbolGraph` / `DependencyGraph`).
