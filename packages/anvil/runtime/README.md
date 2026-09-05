# @eddacraft/anvil-runtime

| Type  | Authority | Owner   | Status | Freshness                                    |
| ----- | --------- | ------- | ------ | -------------------------------------------- |
| Guide | Derived   | EMBERRS | Live   | Last reviewed 2026-09-05 against EMBERRS-001 |

| Upstream                                                                     | Downstream                        |
| ---------------------------------------------------------------------------- | --------------------------------- |
| `packages/anvil/runtime/src/index.ts`, `packages/anvil/runtime/package.json` | API/docs consumers, runtime tests |

This package retains TypeScript cache utilities and feature-flag resolution. The
API and docs shell use the `/feature-flags` subpath. Rust owns anvil engine
execution; the JS watch and agent/lock/queue implementations and exports were
removed under [EMBERRS-001](../../../plans/modules/ember-rust-migration.aps.md).

## Supported entry points

| Export                                   | Purpose                                         |
| ---------------------------------------- | ----------------------------------------------- |
| `@eddacraft/anvil-runtime`               | Cache utilities                                 |
| `@eddacraft/anvil-runtime/cache`         | Cache providers                                 |
| `@eddacraft/anvil-runtime/feature-flags` | API/docs flag resolver, snapshots and telemetry |

There is no `/watch` or concurrency API. The cache retains a private atomic
text-write helper; it is not a queue manager or agent coordinator. Builds clean
old output so removed modules cannot survive in a reused distribution folder.

## Validation

```bash
pnpm --filter @eddacraft/anvil-runtime build
pnpm --filter @eddacraft/anvil-runtime test
```

The test command checks the retirement boundary before running Vitest.
Historical implementations remain in Git history, not in shipped source/build
exports.
