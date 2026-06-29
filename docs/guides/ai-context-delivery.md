# AI Context Delivery

| Type  | Authority     | Owner | Status | Freshness                                                                                                         |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | GCTX  | Live   | Last reviewed 2026-06-26 against `docs/architecture/graph-context-delivery-spec.md` and the live MCP tool surface |

| Upstream                                                                                              | Downstream                                                         |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `docs/architecture/graph-context-delivery-spec.md`, `plans/decisions/083-gctx-mcp-delivery-target.md` | `docs/public/anvil/integrations/mcp.md`, assistant/MCP integrators |

## Overview

Anvil exposes a read-only **graph-context** surface over MCP: a set of tools and
resources that let an AI assistant (Claude Code, Cursor) query a map of your
codebase — symbols, dependencies, callers, and the blast radius of a change —
instead of blindly reading whole files. The graph is built and kept warm by the
`anvil-intercept` daemon; the MCP server projects identity-only answers from it.

This context is **optional and advisory**. It never changes your code and never
blocks an action. If you are looking for the gate that validates a write before
it lands, that is a different surface — see
[Graph context is not launch validation](#graph-context-is-not-launch-validation)
first, because conflating the two is the most common source of confusion.

## Graph context is not launch validation

Anvil's MCP surface has two distinct halves. Keep them separate:

- **Launch validation (`anvil_validate_write`)** is the enforcement gate. An
  assistant calls it _before_ applying a write; it returns a `decision` based on
  secrets, anti-patterns, and boundary rules. The vocabulary is four-valued:
  `block` is authoritative (do not write), `warn` means findings were detected
  but the workspace's enforcement mode lets the write proceed (surface them and
  continue), `gateUnavailable` means the gate could not run — e.g. credentials
  missing or backend offline — (surface the warning and proceed), and `allow`
  means it passed. It is launch-critical: it exists to stop bad writes landing.
  See [Save-time validation](../public/anvil/guides/save-time-validation.md) and
  the [Agent harness guide](../public/anvil/guides/agent-harness.md).
- **Graph context (this guide)** is read-only projection. The tools below answer
  questions about the codebase so the assistant can reason better. They return
  context, never a decision, and never block.

A useful rule of thumb: reach for **`anvil_validate_write`** when you are about
to _change_ code, and for the **graph-context tools** when you are trying to
_understand_ it. They are complementary — an assistant typically pulls context
to plan an edit, then validates the edit before writing.

> The architecture docs and team shorthand sometimes call launch validation
> "RMCP validation" (the Rust MCP full-port server that hosts it). The
> user-facing entry point is the `anvil_validate_write` tool described here.

## Supported clients and setup

The graph-context surface ships with the Rust CLI's MCP server,
`anvil mcp serve --stdio`, and is verified against **Claude Code** and
**Cursor**. The daemon must be running for the tools to return data.

The one-step install writes the client config for you:

```bash
# Start Anvil (auto-installs the MCP server; prompts for consent)
anvil start

# …or install for a specific client explicitly
anvil mcp install --client claude-code
anvil mcp install --client cursor
```

If you prefer to wire the server by hand, add the `anvil mcp serve --stdio` shim
to your client's MCP config — `~/.claude.json` for Claude Code,
`~/.cursor/mcp.json` for Cursor. The Claude Code entry needs a `"type": "stdio"`
discriminator:

```json
{
  "mcpServers": {
    "anvil": {
      "type": "stdio",
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"],
      "env": {}
    }
  }
}
```

Cursor uses the same `mcpServers` entry without the `type` field. The
[MCP integration guide](../public/anvil/integrations/mcp.md) has the exact
per-client shapes, and `anvil mcp install` writes them for you.

Restart the client (or reload its MCP servers from its settings) after writing
the configuration, or the new tools will not appear.

**Verify the connection.** From your client, read the `graph://stats` resource
(or call `anvil_search_symbols` with any short name). A `ready` result confirms
the wiring works — even zero counts are fine (an empty or still-warming
workspace is legitimate). A `not_ready` outcome means the graph is still
warming; retry shortly. An `unavailable` response means the daemon is not
running: check `anvil intercept status`, and `anvil start` to (re)start it and
re-trigger graph warming.

## The tools

All six tools are **identity-only by default**: they return symbol identities
(name, kind, workspace-relative path, visibility) and edge topology — never
source text, absolute paths, or secrets. Results are deterministic. The listing
tools — `anvil_search_symbols`, `anvil_find_dependents`, and
`anvil_find_callers` — paginate with opaque cursors; `anvil_impact_of_change`,
`anvil_affected_tests`, and `anvil_symbol_context` instead return a single
bounded report.

- **`anvil_search_symbols`** — find symbols by name (case-insensitive
  substring), kind, file, language, or visibility. The entry point for "where is
  X?".
- **`anvil_find_dependents`** — the files that import a given file (its blast
  radius), each with a hop distance (`1` = direct importer, `2` = transitive).
- **`anvil_find_callers`** — the symbols that call a given symbol. A best-effort
  static over-approximation: it marks overload fan-out as `heuristic` and an
  incomplete walk as `partial`, and cannot see dynamic dispatch — do not treat
  it as an authoritative caller set.
- **`anvil_impact_of_change`** — given changed file _paths_ (never diff content;
  up to 200 files), the affected symbols, the depth-bounded set of dependent
  files, and a best-effort `known_tests` subset. "What might break if I change
  these?".
- **`anvil_affected_tests`** — given changed file paths, the test files that
  import them (with their evidence edges) plus `coverage_gaps` (changed non-test
  files with no test importer). Relevance is an import heuristic, marked
  `heuristic: true` — not execution-verified coverage.
- **`anvil_symbol_context`** — bounded context around a seed symbol or file:
  neighbourhood symbols, one-hop importers, and (for symbol seeds) direct
  callers, each with a span-as-location under a token budget. This is the
  one-call "tell me about this symbol" tool, and the only one that can return
  source snippets (see [Privacy and redaction](#privacy-and-redaction)).

The reverse-dependency tools cap their walk at **2 hops**, matching the daemon's
impact-depth limit.

## The `graph://` resources

For clients that prefer MCP resources to tool calls, three identity-only
resources expose the resident graph directly:

- **`graph://stats`** — counts only (symbol, edge, and file totals). The
  lowest-risk surface; a good warm-up probe.
- **`graph://symbols`** — all resident symbols as identity summaries. Scope with
  `?file=…` or page with `?cursor=…`.
- **`graph://edges`** — the resident symbol-graph edges as
  `(from, to, edge_type)` summaries. Scope with `?file=…` or page with
  `?cursor=…` (and `?limit=…`). The response carries a `bounded` flag indicating
  whether the daemon's walk was capped by its node budget — it is a result
  field, not a request parameter.

Resources and tools share one per-session egress budget, so a client cannot
reassemble the whole graph past the ceiling by alternating between them.

## Privacy and redaction

The default posture is **identity-only**. Across every tool and resource, the
following never crosses the boundary: source text, absolute paths, secrets,
session/worktree or operator identity (usernames, hostnames, PIDs), and raw
trust levels. What you get is the shape of the code — names, kinds,
workspace-relative paths, and edges — not its contents.

Source **snippets** are an explicit opt-in, and only `anvil_symbol_context` can
return them. They are double-gated: the operator must enable egress for the
workspace **and** the request must set `includeSource: true`. With either
missing, the tool returns identity-only locations — and, when source was
requested but egress is off, a `snippetEgressHint` telling the assistant how to
ask the operator to enable it.

The recommended way to opt in is per-workspace and persisted:

```bash
anvil gctx egress enable     # prints the consequence, asks for confirmation
anvil gctx egress status     # shows the effective state and where it comes from
anvil gctx egress disable    # revert to identity-only
```

`enable` records consent under the workspace's ignored
`anvil/witness/gctx-egress.json` (after an explicit confirmation — pass `--yes`
to acknowledge non-interactively). The consent is per-workspace, so enabling it
in one repo does not affect another.

`ANVIL_GCTX_EGRESS` remains a **process-scoped override** that takes precedence
over the persisted consent, and it is re-read on every call:

- **unset** (the default) — the persisted workspace consent decides; with no
  consent recorded, the surface is identity-only.
- **`1`** — snippets are permitted for this process regardless of persisted
  consent (still per-request via `includeSource`).
- **`0`** — a hard kill-switch: the entire graph-context surface is off and
  every tool and resource returns a `disabled` outcome, overriding any persisted
  consent.

So "off by default" refers to _snippets_, not the surface. Do not set
`ANVIL_GCTX_EGRESS=0` expecting "the safe default" — that takes the whole
surface offline; leaving it unset (with no consent recorded) is the safe,
identity-only default.

When snippets are enabled, they still pass a deny-by-default pipeline before any
text is emitted: sensitive paths (`.env*`, `*.pem`/`*.key`, `.git/`, `secrets/`,
`.ssh/`, …) are dropped entirely, gitignored files are withheld, and a secret
scan redacts matches in the emitted text. Counts of what was dropped or redacted
are reported; the dropped content is not.

For the command reference see the
[CLI surface reference](../runbooks/cli-surface.md) (`anvil gctx`); for the
underlying flag see the [Feature flag reference](feature-flag-reference.md)
(`gctx.egress`).

## Graph states an assistant will see

Because the graph is daemon-backed and warms in the background, a tool may
return a **named outcome** instead of a result. Assistants should handle each:

- **`ready`** — the graph is readable; results follow (possibly empty). If the
  graph is `stale`, the result still comes back, flagged.
- **`not_ready`** — the graph is warming or cold. Recoverable: a request
  triggers an on-demand warm-up, so a retry usually succeeds — a few seconds for
  an incremental warm-up, up to roughly a minute for a first full scan of a
  large workspace. A `recovery_hint` describes the state.
- **`unavailable`** — the daemon is unreachable or no graph exists. Not
  recoverable by retry: check `anvil intercept status`, and if it is not
  running, `anvil start` will (re)start it and re-trigger warming.
- **`disabled`** — an operator has switched the surface off
  (`ANVIL_GCTX_EGRESS=0` — see [Privacy and redaction](#privacy-and-redaction)).
  Respect it; do not retry.
- **`invalid_query`** — the request was rejected before any read (for example a
  path-traversal or scheme-prefixed input, or an over-cap parameter). The
  `reason` says what to fix; correct the parameters and retry.

`anvil_symbol_context` can additionally return two budget outcomes, each
carrying **partial** results up to the limit (not an error): `bounded` when the
caller's token budget truncated the slice, and `budget_exceeded` when the
per-session snippet byte ceiling did. To get a complete response, narrow the
scope or disable snippets.

## Example workflows

- **"What breaks if I change these files?"** — `anvil_impact_of_change` for the
  affected symbols and dependent files, then `anvil_affected_tests` for the
  tests to run and the coverage gaps to watch.
- **"Help me understand this symbol before I edit it."** —
  `anvil_symbol_context` for the neighbourhood, importers, and callers in one
  call; enable snippets if you want the source inline.
- **"Who calls this function?"** — `anvil_find_callers` (symbol-level), or
  `anvil_find_dependents` for the file-level blast radius.
- **"Where is the symbol named `foo`?"** — `anvil_search_symbols`.

A worked "safe refactor" sequence ties them together:

1. `anvil_search_symbols` with `name: "parsePayload"` → locate the symbol and
   its file.
2. `anvil_impact_of_change` with `changedFiles: ["src/parse.ts"]` → the affected
   symbols and the dependent files that import it.
3. `anvil_affected_tests` with the same `changedFiles` → the tests to run and
   any `coverage_gaps` to flag.
4. Make the edit, then `anvil_validate_write` on the new content before writing.

In a typical edit loop the assistant pulls context with these tools to plan the
change, then calls `anvil_validate_write` before applying it.

## See also

- [MCP integration guide](../public/anvil/integrations/mcp.md) — full client
  setup and the complete tool/resource reference
- [Save-time validation](../public/anvil/guides/save-time-validation.md) — the
  `anvil_validate_write` enforcement path
- [Graph context delivery spec](../architecture/graph-context-delivery-spec.md)
  — the delivery contract and egress rules (CE-1..CE-12)
- [Feature flag reference](feature-flag-reference.md) — operating `gctx.egress`
