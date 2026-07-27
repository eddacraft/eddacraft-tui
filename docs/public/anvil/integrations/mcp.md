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

Write or verify the MCP entry for a guided client:

```text
anvil mcp install --client cursor
anvil mcp install --client claude-code --verify
```

The first command installs; the second only verifies an existing entry. Prefer
`--verify` after an upgrade rather than reinstalling blindly.

The generated [support reference](../reference/support.md) documents the guided
activation pair for the current public release.

### Expanded client registry (newer betas)

After 0.9.0-beta, `anvil mcp install --client` can accept a wider set of client
ids (for example Codex, OpenCode, Gemini CLI, VS Code, Copilot CLI, Grok, Warp,
and Zed, alongside Cursor and Claude Code). Each installer writes that client's
documented config shape and leaves unmanaged third-party entries intact.

Discover what **your** binary supports:

```text
anvil mcp install --help
```

Use only client ids and flags listed there. Do not assume a client from a newer
release note is present in an older binary.

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
coverage, or check [managed agent skills](skills.md) when your binary exposes
them.
