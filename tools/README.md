# Tools

> **Status:** Placeholder for v1.1+

Development tools, generators, and infrastructure.

## Structure

```
tools/
├── generators/   # Nx generators for scaffolding
├── scripts/      # Build and utility scripts (migrate from scripts/)
└── docker/       # Docker configurations and compose files
```

## Migration Plan

| Directory  | Source          |
| ---------- | --------------- |
| generators | New             |
| scripts    | Root `scripts/` |
| docker     | New             |

## Generators (Planned)

- `anvil:package` - Create new @anvil/\* package
- `anvil:adapter` - Create new adapter package
- `anvil:app` - Create new application
