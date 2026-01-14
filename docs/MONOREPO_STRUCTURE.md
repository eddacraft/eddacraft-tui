# Monorepo Structure

This document describes the current and target monorepo structure.

## Current Structure (v1.0)

```
anvil/
├── cli/                 # @anvil/cli - Command-line interface
├── core/                # @anvil/core - Schema, validation, gates
├── ui/                  # @anvil/ui - UI components
├── packs/               # @anvil/packs - Pack definitions
├── packages/
│   ├── adapters/        # @anvil/adapters - Format converters
│   ├── aps/             # @anvil/aps - APS parser
│   ├── eslint-plugin-anvil/
│   └── vscode-extension/
├── e2e/                 # Playwright E2E tests
├── docs/                # Internal documentation
├── plans/               # APS planning specs
└── scripts/             # Build utilities
```

## Target Structure (v1.1+)

```
anvil/
├── apps/                    # Deployable applications
│   ├── anvil-cli/          # CLI (migrate from cli/)
│   ├── anvil-api/          # REST/GraphQL API
│   ├── anvil-ui/           # Web UI
│   ├── website/            # Marketing site
│   ├── docs-site/          # Public documentation
│   └── e2e/                # E2E test suites
│
├── packages/
│   ├── anvil/              # Core domain (split from core/)
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

| Component | Current              | Target                | Status            |
| --------- | -------------------- | --------------------- | ----------------- |
| CLI       | `cli/`               | `apps/anvil-cli/`     | Placeholder ready |
| Core      | `core/`              | `packages/anvil/*`    | Placeholder ready |
| Adapters  | `packages/adapters/` | `packages/adapters/*` | Placeholder ready |
| API       | -                    | `apps/anvil-api/`     | Placeholder ready |
| UI        | `ui/`                | `apps/anvil-ui/`      | Placeholder ready |
| Website   | -                    | `apps/website/`       | Placeholder ready |
| Docs site | -                    | `apps/docs-site/`     | Placeholder ready |
| E2E       | `e2e/`               | `apps/e2e/*`          | Placeholder ready |
| Scripts   | `scripts/`           | `tools/scripts/`      | Placeholder ready |

## Migration Plan

The full migration is planned for v1.1. See:

- `docs/planning/monorepo-cleanup-impact-assessment.md` - Full impact analysis
- `plans/index.aps.md` - Release roadmap

## Workspace Configuration

The `pnpm-workspace.yaml` already includes the target paths, so new packages can
be created in the target locations immediately.
