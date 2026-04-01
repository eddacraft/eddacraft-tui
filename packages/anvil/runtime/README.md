# @eddacraft/anvil-runtime

Runtime orchestration and I/O layer for the Anvil system. Handles gate
execution, file watching, caching, constraint export, and multi-agent
concurrency coordination -- everything that `@eddacraft/anvil-core` delegates
for I/O-heavy operations.

## Status

Winding down -- the Rust CLI and kernel have replaced the primary execution
paths. This package remains in use by the MCP server and e2e tests.

## API Surface

| Export | Description |
| --- | --- |
| `@eddacraft/anvil-runtime` | Everything below |
| `@eddacraft/anvil-runtime/gate` | Gate runner and check orchestration |
| `@eddacraft/anvil-runtime/cache` | Cache providers |
| `@eddacraft/anvil-runtime/watch` | File watcher, git status, debouncer |
| `@eddacraft/anvil-runtime/export` | Constraint export (llms.txt, MCP resource, prompt fragment) |

Also exports the full concurrency module: agent management, lock management,
queue management, git agent identification, and atomic file operations.

## Consumers

- `@eddacraft/anvil-mcp-server`
- `@eddacraft/anvil-cli`
- e2e tests

## Development

```bash
pnpm --filter @eddacraft/anvil-runtime build
pnpm --filter @eddacraft/anvil-runtime test
```
