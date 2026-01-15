# Monorepo Structure

This document describes the current and target monorepo structure.

## Current Structure (migration state)

```
anvil/
├── apps/                # Deployable applications
│   ├── anvil-cli/
│   ├── anvil-api/
│   ├── anvil-ui/
│   ├── website/
│   ├── docs-site/
│   └── e2e/
├── packages/
│   ├── anvil/
│   ├── adapters/
│   ├── aps/
│   ├── edda-stack/
│   ├── eslint-plugin-anvil/
│   ├── kindling-integration/
│   ├── platform/
│   ├── shared/
│   ├── tooling/
│   └── vscode-extension/
├── tools/               # Generators and scripts
├── docs/                # Internal documentation
└── plans/               # APS planning specs
```

## Target Structure (v1.1+)

```
anvil/
├── apps/                    # Deployable applications
│   ├── anvil-cli/          # CLI application
│   ├── anvil-api/          # REST/GraphQL API
│   ├── anvil-ui/           # Web UI
│   ├── website/            # Marketing site
│   ├── docs-site/          # Public documentation
│   └── e2e/                # E2E test suites
│
├── packages/
│   ├── anvil/              # Core domain
│   │   ├── contracts/      # Schemas, events, types
│   │   ├── ports/          # Interfaces
│   │   ├── core/           # Pure domain logic
│   │   ├── runtime/        # Orchestration
│   │   ├── policy/         # OPA/Rego wrappers
│   │   └── sdk/            # Client SDK
│   │
│   ├── edda-stack/         # Memory/proposal system
│   │   ├── contracts/
│   │   ├── ports/
│   │   ├── ember/
│   │   ├── edda/
│   │   └── testing/
│   │
│   ├── adapters/           # Per-integration adapters
│   ├── platform/           # Cross-cutting infrastructure
│   ├── shared/             # Shared utilities
│   └── tooling/            # Build configurations
│
├── tools/                  # Nx generators and scripts
├── docs/                   # Internal documentation
└── plans/                  # APS planning specs
```

## Migration Status

| Component | Current              | Target                | Status   |
| --------- | -------------------- | --------------------- | -------- |
| CLI       | `apps/anvil-cli/`    | `apps/anvil-cli/`     | In place |
| Core      | `packages/anvil/*`   | `packages/anvil/*`    | In place |
| Adapters  | `packages/adapters/` | `packages/adapters/*` | In place |
| API       | `apps/anvil-api/`    | `apps/anvil-api/`     | In place |
| UI        | `apps/anvil-ui/`     | `apps/anvil-ui/`      | In place |
| Website   | `apps/website/`      | `apps/website/`       | In place |
| Docs site | `apps/docs-site/`    | `apps/docs-site/`     | In place |
| E2E       | `apps/e2e/`          | `apps/e2e/*`          | In place |
| Scripts   | `tools/scripts/`     | `tools/scripts/`      | In place |

## Migration Plan

The full migration is planned for v1.1. See:

- `docs/planning/monorepo-cleanup-impact-assessment.md` - Full impact analysis
- `plans/index.aps.md` - Release roadmap

## Workspace Configuration

The `pnpm-workspace.yaml` tracks `apps/*`, `packages/*`, and `tools/*` after
removing legacy roots, so new packages can be created in the target locations
immediately.
