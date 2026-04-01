# @eddacraft/anvil-ports

Interface definitions (ports) for the Anvil system. Depends only on
`@eddacraft/anvil-core` for types. Consumers provide their own implementations.

## Status

Winding down -- the Rust kernel defines its own trait boundaries. This package
remains in use by the TypeScript runtime and MCP server.

## API Surface

| Interface | Description |
| --- | --- |
| `IStorageProvider` | File system abstraction (read, write, exists, delete, list, mkdir) |
| `ICacheProvider` | Cache abstraction (get, set, has, delete, clear) |
| `ICheckRunner` | Gate check execution interface |
| `IConfigProvider` | Configuration loading interface |

## Consumers

- `@eddacraft/anvil-runtime`
- `@eddacraft/shared-storage`
- `@eddacraft/anvil-mcp-server`

## Development

```bash
pnpm --filter @eddacraft/anvil-ports build
pnpm --filter @eddacraft/anvil-ports test
```
