# anvil developer functions — full reference

Deep detail for the developer functions. The loop and everyday usage live in
[`../SKILL.md`](../SKILL.md); reach here for exact parameters, the `graph://`
resources, and the egress/redaction mechanics.

## Setup and verification

The graph-context surface ships with the Rust CLI's MCP server,
`anvil mcp serve --stdio`, verified against Claude Code and Cursor. The daemon
must be running for the tools to return data.

```bash
anvil start                              # auto-installs the MCP server, starts the daemon
anvil mcp install --client claude-code   # install for Claude Code explicitly
anvil mcp install --client cursor        # install for Cursor explicitly
```

Manual wiring — add the shim to the client's MCP config (`~/.claude.json` for
Claude Code, which needs the `"type": "stdio"` discriminator;
`~/.cursor/mcp.json` for Cursor, same entry without `type`):

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

Restart the client (or reload its MCP servers) after writing the config, or the
tools will not appear. Verify by reading `graph://stats` or calling
`anvil_search_symbols` — a `ready` result confirms the wiring.

## Tool details

All six graph-context tools are **identity-only by default**: they return symbol
identities (name, kind, workspace-relative path, visibility) and edge topology —
never source text, absolute paths, or secrets. Results are deterministic. The
listing tools (`anvil_search_symbols`, `anvil_find_dependents`,
`anvil_find_callers`) paginate with opaque cursors; the report tools
(`anvil_impact_of_change`, `anvil_affected_tests`, `anvil_symbol_context`)
return a single bounded report. The reverse-dependency walks cap at **2 hops**,
matching the daemon's impact-depth limit.

| Tool                     | Key inputs                                                   | Returns                                                                                    |
| ------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `anvil_search_symbols`   | `name` (substring), `kind`, `file`, `language`, `visibility` | Matching symbol identities, paginated                                                      |
| `anvil_find_dependents`  | a file path                                                  | Importing files, each with hop distance (`1` direct, `2` transitive)                       |
| `anvil_find_callers`     | a symbol                                                     | Calling symbols; flags `heuristic` (overload fan-out) and `partial` (incomplete walk)      |
| `anvil_impact_of_change` | `changedFiles` (paths, ≤200; never diff content)             | Affected symbols, depth-bounded dependent files, best-effort `known_tests`                 |
| `anvil_affected_tests`   | `changedFiles` (paths)                                       | Test files importing them (with evidence edges) + `coverage_gaps`; `heuristic: true`       |
| `anvil_symbol_context`   | a seed symbol or file; token budget; `includeSource`         | Neighbourhood symbols, one-hop importers, direct callers (symbol seeds), spans-as-location |

`anvil_find_callers` is a best-effort static over-approximation and cannot see
dynamic dispatch — never treat its output as an authoritative caller set.
`anvil_affected_tests` relevance is an import heuristic, not execution-verified
coverage.

## `graph://` resources

For clients that prefer resources to tool calls:

- **`graph://stats`** — counts only (symbol, edge, file totals). Lowest-risk
  surface; a good warm-up probe.
- **`graph://symbols`** — all resident symbols as identity summaries. Scope with
  `?file=…` or page with `?cursor=…`.
- **`graph://edges`** — resident symbol-graph edges as `(from, to, edge_type)`
  summaries. Scope with `?file=…`, page with `?cursor=…` and `?limit=…`. The
  response carries a `bounded` flag indicating whether the daemon's walk was
  capped by its node budget — a result field, not a request parameter.

Resources and tools share one per-session egress budget, so a client cannot
reassemble the whole graph past the ceiling by alternating between them.

## `anvil_validate_write`

The pre-write enforcement gate (the Rust shim, daemon-backed when reachable;
embedded fallback otherwise). Call it before applying a write.

```json
{
  "tool": "anvil_validate_write",
  "arguments": {
    "workspaceRoot": "/absolute/path/to/project",
    "path": "src/auth/login.ts",
    "operation": "create",
    "proposedContent": "export const login = …"
  }
}
```

The `decision` is four-valued: `block` (authoritative — do not write), `warn`
(findings detected but enforcement mode permits the write — surface and
proceed), `gateUnavailable` (gate could not run — surface and proceed), `allow`
(passed). The response also carries a `correlation` envelope whose
`daemonStatus` reports whether the daemon-backed path ran (`available`), fell
back to the embedded scanner (`unavailable`), or was not compiled in
(`not-wired`).

`anvil_status` is a read-only workspace-health summary (status, available
checks, config, baseline presence, version) with path values redacted to
workspace-relative forms.

> Team shorthand sometimes calls launch validation "RMCP validation" (the Rust
> MCP full-port server that hosts it). The user-facing entry point is
> `anvil_validate_write`.

## Egress and redaction

The default posture is **identity-only**. Source **snippets** are an explicit
opt-in and only `anvil_symbol_context` can return them. They are double-gated:
the operator enables egress for the workspace **and** the request sets
`includeSource: true`. With either missing, the tool returns identity-only
locations — and, when source was requested but egress is off, a
`snippetEgressHint` describing how to ask the operator to enable it.

Operators opt in per-workspace and persisted:

```bash
anvil gctx egress enable     # prints the consequence, asks for confirmation
anvil gctx egress status     # shows the effective state and where it comes from
anvil gctx egress disable    # revert to identity-only
```

`enable` records consent under the workspace's ignored
`anvil/witness/gctx-egress.json` (pass `--yes` to acknowledge
non-interactively). Consent is per-workspace.

`ANVIL_GCTX_EGRESS` is a process-scoped override, re-read on every call:

- **unset** (default) — persisted workspace consent decides; with none recorded,
  the surface is identity-only.
- **`1`** — snippets permitted for this process regardless of persisted consent
  (still per-request via `includeSource`).
- **`0`** — a hard kill-switch: the entire surface is off and every tool and
  resource returns `disabled`, overriding persisted consent.

"Off by default" refers to _snippets_, not the surface. Setting
`ANVIL_GCTX_EGRESS=0` takes the whole surface offline; the safe identity-only
default is leaving it unset.

When snippets are enabled they still pass a deny-by-default pipeline before any
text is emitted: sensitive paths (`.env*`, `*.pem`/`*.key`, `.git/`, `secrets/`,
`.ssh/`, …) are dropped, gitignored files are withheld, and a secret scan
redacts matches in the emitted text. Counts of what was dropped or redacted are
reported; the dropped content is not.

## Source docs (anvil repo — maintainers only)

Internal anvil-repo paths — not available in consuming projects; for maintainers
only. These are the upstream authorities this skill is distilled from; consult
them in the anvil repository if behaviour seems to have changed:

- `docs/guides/ai-context-delivery.md` — the graph-context surface and egress
- `docs/public/anvil/integrations/mcp.md` — client setup and tool reference
- `docs/public/anvil/guides/save-time-validation.md` — the
  `anvil_validate_write` enforcement path
