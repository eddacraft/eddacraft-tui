---
id: mcp
title: Model Context Protocol integration
description:
  Understand and verify anvil's local connection to supported AI clients.
---

# Model Context Protocol integration

Model Context Protocol (MCP) lets a supported AI client call anvil locally
before writing a proposed change.

## Guided activation clients

The guided path in `anvil start` configures **Cursor** and **Claude Code** when
they are detected. That path stays consent-first: nothing is written unless you
select it.

```text
anvil start
```

To prepare both guided clients even when one is not currently detected:

```text
anvil start --all-mcp-clients
```

To skip client configuration entirely:

```text
anvil start --no-mcp
```

## Install a specific client

`anvil mcp install --client` writes that client's documented config shape and
leaves unmanaged third-party entries intact. Supported client ids include:

`claude-code`, `cursor`, `codex`, `opencode`, `gemini-cli`, `antigravity`,
`openclaw`, `vscode`, `copilot-cli`, `grok`, `warp`, and `zed`.

```text
anvil mcp install --client codex
anvil mcp install --client vscode --dry-run
anvil mcp install --client zed --scope project --verify
```

Useful flags:

- `--verify` — check the existing entry without writing.
- `--dry-run` — preview the path and entry without writing.
- `--scope global|project` — choose install location (global is the beta
  default; some clients are project-only).

To reach the wider registry from activation instead of the guided pair, pass
`--mcp-client <id>` (repeatable), `--all-mcp-clients`, or set
`ANVIL_ALL_MCP_CLIENTS`. Prefer `anvil mcp install --client` when you want one
explicit client.

The generated [support reference](../reference/support.md) documents the guided
activation pair. Use `anvil mcp install --help` for the full client list on your
installed binary.

## Verify protection

Restart the client when activation asks, then run:

```text
anvil start --verify
```

Only `protecting` proves the current pre-write path. A configured file or
running daemon alone is not enough.

## Inspect MCP commands

```text
anvil mcp --help
```

The current CLI provides `install` and `serve`. Connection readiness is reported
by `anvil start --verify`, not by a separate MCP status command. Subcommands can
evolve during beta — prefer installed help over hand-copied configuration
shapes.

## Security boundary

The MCP server runs locally and validates requests against the current project
and user boundary. Do not expose it as a network service or copy credentials
into client configuration.

Graph context shared through a configured client is identity-only by default.
See [local data and security](../operations/security.md) before enabling source
snippet egress.

## Next step

Use [save-time validation](../guides/save-time-validation.md) as fallback
coverage, or install [managed agent skills](skills.md) for clients that support
them.
