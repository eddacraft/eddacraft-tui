# @eddacraft/anvil-runtime

| Type  | Authority | Owner   | Status | Freshness                                |
| ----- | --------- | ------- | ------ | ---------------------------------------- |
| Guide | Derived   | EMBERRS | Live   | Last reviewed 2026-09-11 against CIB-418 |

| Upstream                                                                     | Downstream                        |
| ---------------------------------------------------------------------------- | --------------------------------- |
| `packages/anvil/runtime/src/index.ts`, `packages/anvil/runtime/package.json` | API/docs consumers, runtime tests |

This package provides feature-flag resolution for the API and docs shell through
the `/feature-flags` subpath. Rust owns anvil engine execution; the JS watch and
agent/lock/queue implementations and exports were removed under
[EMBERRS-001](../../../plans/modules/ember-rust-migration.aps.md). The unused
TypeScript cache implementation and `/cache` export were retired under CIB-418.

## Supported entry points

| Export                                   | Purpose                                         |
| ---------------------------------------- | ----------------------------------------------- |
| `@eddacraft/anvil-runtime`               | Empty compatibility root                        |
| `@eddacraft/anvil-runtime/feature-flags` | API/docs flag resolver, snapshots and telemetry |

There is no cache, `/watch`, or concurrency API. Builds clean old output so
removed modules cannot survive in a reused distribution folder.

## Validation

```bash
pnpm --filter @eddacraft/anvil-runtime build
pnpm --filter @eddacraft/anvil-runtime test
```

The test command checks the retirement boundary before running Vitest.
Historical implementations remain in Git history, not in shipped source/build
exports.
