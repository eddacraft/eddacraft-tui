# Legacy Core Archive

This directory contains the original `core/` package code before the monorepo
migration.

## Migration Status

The code from this directory has been migrated to the new package structure:

| Original Location        | New Location                                                  |
| ------------------------ | ------------------------------------------------------------- |
| `core/src/schema/`       | `packages/anvil/contracts/src/schemas/`                       |
| `core/src/types/`        | `packages/anvil/contracts/src/types/`                         |
| `core/src/architecture/` | `packages/anvil/core/src/architecture/`                       |
| `core/src/antipattern/`  | `packages/anvil/core/src/antipattern/`                        |
| `core/src/drift/`        | `packages/anvil/core/src/drift/`                              |
| `core/src/explain/`      | `packages/anvil/core/src/explain/`                            |
| `core/src/provenance/`   | `packages/anvil/core/src/provenance/`                         |
| `core/src/suppression/`  | `packages/anvil/core/src/suppression/`                        |
| `core/src/validation/`   | `packages/anvil/core/src/validation/`                         |
| `core/src/warnings/`     | `packages/anvil/core/src/warnings/`                           |
| `core/src/cache/`        | `packages/anvil/runtime/src/cache/`                           |
| `core/src/watch/`        | `packages/anvil/runtime/src/watch/`                           |
| `core/src/export/`       | `packages/anvil/runtime/src/export/`                          |
| `core/src/gate/`         | `packages/anvil/runtime/src/gate/` + `packages/anvil/policy/` |
| `core/src/crypto/`       | `packages/anvil/core/src/crypto/`                             |
| `core/src/utils/`        | `packages/anvil/core/src/utils/`                              |

## Purpose

This archive is kept for reference during the migration transition period. Once
the migration is fully validated and stable, this directory can be deleted.

## Do Not Use

**Do not import from this location.** Use the new package imports:

- `@anvil/contracts` - Schemas and types
- `@anvil/ports` - Interface definitions
- `@anvil/core` - Pure domain logic
- `@anvil/runtime` - Execution and orchestration
- `@anvil/policy` - OPA policy evaluation
