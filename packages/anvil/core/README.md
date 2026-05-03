# @eddacraft/anvil-core

Core domain logic for the Anvil system. Contains schemas, types, analysis
engines, and validation -- everything that does not require heavy I/O. Some
modules perform lightweight filesystem operations (provenance, drift snapshots);
heavy orchestration lives in `@eddacraft/anvil-runtime`.

## Status

Winding down -- the Rust crates (`anvil-kernel`, `anvil-checks`) have replaced
the performance-critical paths. This package remains in use by the MCP server
and e2e tests.

## API Surface

Subpath exports for targeted imports:

| Export                               | Description                                                  |
| ------------------------------------ | ------------------------------------------------------------ |
| `@eddacraft/anvil-core`              | Everything below                                             |
| `@eddacraft/anvil-core/architecture` | Architecture analysis (boundaries, file rules, import rules) |
| `@eddacraft/anvil-core/crypto`       | Hashing and integrity utilities                              |
| `@eddacraft/anvil-core/explain`      | Human-readable explanations for warnings                     |
| `@eddacraft/anvil-core/provenance`   | Provenance tracking                                          |
| `@eddacraft/anvil-core/validation`   | Schema validation                                            |
| `@eddacraft/anvil-core/warnings`     | Warning utilities                                            |
| `@eddacraft/anvil-core/utils`        | General utilities                                            |

Scanner-era subpaths for antipattern, drift, and suppression were removed in
`0.5.1-beta`; use the Rust CLI surfaces for those flows.

Also re-exports all contracts (schemas, types, events) formerly in
`@eddacraft/anvil-contracts` and platform config from
`@eddacraft/anvil-platform-config`.

## Consumers

- `@eddacraft/anvil-runtime`
- `@eddacraft/anvil-mcp-server`
- `@eddacraft/anvil-cli`
- e2e tests

## Development

```bash
pnpm --filter @eddacraft/anvil-core build
pnpm --filter @eddacraft/anvil-core test
```
