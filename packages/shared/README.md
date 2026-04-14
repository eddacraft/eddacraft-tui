# Shared Packages

Cross-cutting infrastructure packages used across the Anvil monorepo. Introduced
by [ADR-023](../../plans/decisions/023-shared-packages-restructure.md) to break
circular dependencies and provide a clean dependency floor beneath domain
packages.

## Status

Active

## Sub-packages

| Package            | Description                                                      |
| ------------------ | ---------------------------------------------------------------- |
| `shared/auth`      | Authentication helpers                                           |
| `shared/storage`   | `IStorageProvider` implementation with path traversal protection |
| `shared/telemetry` | Telemetry utilities                                              |
| `shared/testing`   | Test helpers, fixtures, mocks                                    |
| `shared/types`     | Branded types and type utilities                                 |

## Guidelines

- No dependencies on `@eddacraft/anvil-*` domain packages (port/contract
  packages like `@eddacraft/anvil-ports` are allowed as interface boundaries)
- Pure functions preferred
- Minimal external dependencies
- Well-documented and tested
