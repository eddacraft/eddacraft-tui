---
id: mcp
title: Model Context Protocol integration
description:
  Understand and verify anvil's local connection to supported AI clients.
---

# Model Context Protocol integration

Model Context Protocol (MCP) lets a supported AI client call anvil locally
before writing a proposed change.

## Supported guided clients

The generated [support reference](../reference/support.md) is the current
authority. The guided activation path can configure Cursor and Claude Code.

## Configure detected clients

```text
anvil start
```

To configure every guided client:

```text
anvil start --all-mcp-clients
```

To avoid client configuration entirely:

```text
anvil start --no-mcp
```

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

Subcommands can evolve during beta. Use the installed help rather than copying
configuration shapes by hand.

## Security boundary

The MCP server runs locally and validates requests against the current project
and user boundary. Do not expose it as a network service or copy credentials
into client configuration.

## Next step

Use [save-time validation](../guides/save-time-validation.md) as fallback
coverage.
