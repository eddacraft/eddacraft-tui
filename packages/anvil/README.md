# @anvil/\* Core Packages

> **Status:** Placeholder for v1.1+

Layered architecture packages for the Anvil core domain.

## Structure

```
anvil/
├── contracts/   # Schemas, events, types (Zod definitions)
├── ports/       # Interface definitions (no implementations)
├── core/        # Pure domain logic (no I/O)
├── runtime/     # Orchestration and execution
├── policy/      # OPA/Rego policy wrappers
└── sdk/         # Client SDK for external consumers
```

## Migration Plan

These packages will be created by splitting the current `core/` package:

| New Package | Source                                   |
| ----------- | ---------------------------------------- |
| contracts   | `core/src/schemas/`, `core/src/types/`   |
| ports       | `core/src/interfaces/` (new)             |
| core        | `core/src/domain/`                       |
| runtime     | `core/src/executor/`, `core/src/runner/` |
| policy      | `core/src/opa/`                          |
| sdk         | New package                              |

## Dependency Direction

```
sdk → runtime → core → ports → contracts
         ↓
      policy
```
