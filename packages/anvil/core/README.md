# @eddacraft/anvil-core

Core domain logic for the Anvil system. Contains schemas, types, architecture
analysis, validation helpers, warning utilities, and the `.anvil` format
compiler -- everything that does not require heavy I/O. Heavy orchestration
lives in `@eddacraft/anvil-runtime` and scanner-era flows now live in Rust
crates and the Rust CLI.

## Status

Winding down -- the Rust crates (`anvil-kernel`, `anvil-checks`) have replaced
the performance-critical scanner paths. This package remains active for
architecture analysis, schema/validation utilities, the `.anvil` compiler, and
tests. The TypeScript MCP server is archived; Rust MCP parity is tracked by
RMCPF.

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
- `@eddacraft/anvil-cli`
- e2e tests

## Development

```bash
pnpm --filter @eddacraft/anvil-core build
pnpm --filter @eddacraft/anvil-core test
```
