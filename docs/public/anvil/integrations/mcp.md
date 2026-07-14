---
id: mcp
title: MCP Integration
description: Connect the Rust Anvil MCP server to an AI editor or coding agent.
sidebar_position: 3
---

# MCP Integration

The Anvil binary includes a native Rust MCP server. It gives an AI assistant
pre-write validation, local governance tools, and privacy-bounded graph context
without installing a separate runtime or package.

The server runs over stdio:

```bash
anvil mcp serve --stdio
```

Cursor and Claude Code are the supported automatic-install targets. Other
stdio-capable MCP clients can use the manual configuration below.

## Recommended setup

Run activation from the repository you want to protect:

```bash
anvil start
```

In an interactive run, Anvil shows the detected MCP clients and starts every
candidate unticked. Select the clients you want before applying; pressing Enter
with nothing selected writes no editor configuration. Restart a configured
editor so it launches the new MCP server.

Probe the resulting protection state without writing anything:

```bash
anvil start --verify
```

A `protecting` result means Anvil has observed the pre-write path. A written
config alone is not enough to claim protection.

### Install one client directly

Use the focused installer when you do not need the rest of activation:

```bash
anvil mcp install --client cursor
anvil mcp install --client claude-code
anvil mcp install --client cursor --verify
```

### Generate or verify configuration

`mcp-config` prints the configuration by default and writes only when asked:

```bash
anvil mcp-config --target cursor
anvil mcp-config --target cursor --write
anvil mcp-config --target cursor --verify
```

The accepted targets are `cursor` and `claude-code`. Use `--workspace <path>`
when generating config for a repository other than the current directory.

## Manual configuration

For another MCP client, add the equivalent server entry and make the repository
the process working directory:

```json
{
  "mcpServers": {
    "anvil": {
      "command": "anvil",
      "args": ["mcp", "serve", "--stdio"]
    }
  }
}
```

The recommended editor integration is stdio: spawn `anvil mcp serve --stdio` as
a child process. `anvil mcp-config` also accepts `--transport http` to emit a
`url` entry pointing at a running daemon HTTP endpoint
(`http://127.0.0.1:<port>/mcp`); use that only when you already run the daemon
and want the editor to connect over HTTP rather than spawning a stdio server.

## Tool catalogue

The current Rust registry exposes 14 tools. The server advertises their schemas
to the MCP client, so callers should use discovery instead of hard-coding
parameters from an older example.

### Write validation

| Tool                   | Purpose                                                                                                           |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `anvil_validate_write` | Validate the complete proposed file state before a create, update, delete, or rename.                             |
| `anvil_apply_patch`    | Validate a unified diff before it is applied, scanning the added lines and preserving a compact approval payload. |

Call one of these before every write and honour its decision:

- `allow` — the proposed content passed;
- `warn` — findings exist, but the workspace mode permits the write;
- `block` — authoritative refusal; do not write through another path;
- `gateUnavailable` — the gate could not run, commonly because credentials or
  the backend are unavailable; surface the warning and follow the response's
  `safeDefault`.

`anvil_validate_write` accepts full proposed content, a patch, or a slim preview
plus SHA-256 digest. `anvil_apply_patch` accepts a file path and unified diff.
Both are validation tools: neither writes the file.

### Local governance

| Tool                   | Behaviour                                                                           | Authentication |
| ---------------------- | ----------------------------------------------------------------------------------- | -------------- |
| `anvil_status`         | Read project config, baseline presence, available checks, and local backend status. | Not required   |
| `anvil_check`          | Run the focused file anti-pattern scan.                                             | Not required   |
| `anvil_query_boundary` | Ask whether one file may import another under the architecture baseline.            | Not required   |
| `anvil_gate`           | Run a planless target-file scan or the full configured gate.                        | Required       |
| `anvil_fix`            | Apply one of the supported deterministic anti-pattern fixes.                        | Required       |
| `anvil_suppress`       | Insert a reasoned, time-boxed suppression comment.                                  | Required       |

The mutating tools enforce workspace containment and authenticate before
changing a file. `anvil_gate` also authenticates because it can execute the
repository's configured toolchain.

### Assistant graph context

| Tool                     | Purpose                                                                      |
| ------------------------ | ---------------------------------------------------------------------------- |
| `anvil_search_symbols`   | Find symbols by name, kind, file, language, or visibility.                   |
| `anvil_find_dependents`  | Traverse files that depend on a target file.                                 |
| `anvil_find_callers`     | Find static callers of a symbol, with heuristic and partial-result markers.  |
| `anvil_impact_of_change` | Report affected symbols, dependent files, and known tests for changed paths. |
| `anvil_affected_tests`   | Suggest likely tests and identify coverage gaps for changed files.           |
| `anvil_symbol_context`   | Return a bounded neighbourhood around a symbol or file.                      |

These tools query the daemon's resident graph. If the graph is cold or the
daemon is absent, they return a named `not_ready`, `unavailable`, or `disabled`
outcome instead of inventing an empty result.

Graph responses are identity-only by default: symbol names, kinds,
workspace-relative paths, visibility, and edges. `anvil_symbol_context` can
include source snippets only when both conditions are true:

1. the operator enabled egress for the workspace; and
2. the individual request sets `includeSource: true`.

Manage consent with:

```bash
anvil gctx egress status
anvil gctx egress enable
anvil gctx egress disable
```

Set `ANVIL_GCTX_EGRESS=0` in the MCP server environment to disable the graph
context surface completely.

## Resource catalogue

The Rust server also exposes read-only resources sourced from the same Rust
readers as the CLI and tools.

### Workspace resources

| Resource               | Contents                                                        |
| ---------------------- | --------------------------------------------------------------- |
| `anvil://baseline`     | Architecture baseline and baseline violation snapshot.          |
| `anvil://boundaries`   | Layer definitions and explicit boundary rules.                  |
| `anvil://patterns`     | Built-in anti-pattern catalogue.                                |
| `anvil://suppressions` | Active suppressions and active/expired totals.                  |
| `anvil://config`       | Discovered Anvil config, source, and parse status.              |
| `anvil://constraints`  | Aggregated boundaries, patterns, conventions, and suppressions. |
| `anvil://drift`        | Latest drift snapshots and their comparison state.              |

### Graph resources

| Resource          | Contents                                               |
| ----------------- | ------------------------------------------------------ |
| `graph://stats`   | Counts of symbols, edges, files, and dependency edges. |
| `graph://symbols` | Paged identity-only symbol summaries.                  |
| `graph://edges`   | Paged identity-only graph edges.                       |

`graph://symbols` and `graph://edges` accept paging and file filters in the URI
query. The graph resource and tool paths share a per-server-session egress
budget; reconnecting starts a new session after the client has exhausted it.

## Daemon and fallback behaviour

The MCP server uses the resident Anvil daemon when it is reachable. On Unix it
connects over an owner-only Unix socket; on Windows it uses an owner-only named
pipe. The pre-write tool reports daemon provenance in its `correlation`
envelope.

If the daemon is unavailable, write validation uses the embedded
correctness-equivalent fallback. Graph tools cannot fall back because the
resident graph belongs to the daemon, so they return an explicit unavailable or
not-ready outcome.

On Linux and macOS, an interactive `anvil start` can start or reuse the daemon.
In headless automation, or on Windows, start the foreground daemon explicitly
when you need daemon-backed validation:

```bash
anvil intercept start --foreground
```

Inspect it from another terminal:

```bash
anvil intercept status
```

## Troubleshooting

1. Run `anvil mcp install --client cursor --verify` (or `claude-code`), or use
   `anvil mcp-config --target cursor --verify` for the matching config target.
2. Restart the editor after changing its MCP configuration.
3. Run `anvil start --verify` and trust the literal protection state.
4. Run `anvil intercept status` if daemon assurance is unavailable.
5. Confirm the editor launches `anvil mcp serve --stdio` with the repository as
   its working directory.

The MCP process pins its workspace at startup. Paths outside that workspace are
rejected, and graph resources never accept a client-supplied workspace root.

For the daemon routing and fallback model, see
[Save-time Validation](../guides/save-time-validation.md). For agent-side
composition patterns, see [Agent Harness Patterns](../guides/agent-harness.md).
