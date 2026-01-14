# Monorepo Migration

> **Module ID:** MONO
> **Status:** Ready
> **Release:** v1.1
> **Dependencies:** None (infrastructure change)

## Overview

Restructure the Anvil monorepo from flat layout to a layered apps/packages
architecture. This migration establishes clear boundaries between deployable
applications and reusable packages, enables better code sharing, and prepares
the codebase for scaling.

**Why now (v1.1):** The current flat structure works for v1.0 but will create
friction as we add API, UI, and additional adapters. Migrating after go-live
lets us ship quickly while preparing for growth.

## Current State

```
anvil/
├── cli/                 # @anvil/cli
├── core/                # @anvil/core (monolithic)
├── ui/                  # @anvil/ui (minimal)
├── packs/               # @anvil/packs
├── packages/
│   ├── adapters/        # @anvil/adapters (bundled)
│   ├── aps/             # @anvil/aps
│   ├── eslint-plugin-anvil/
│   └── vscode-extension/
├── e2e/                 # Playwright tests
├── scripts/             # Build utilities
└── docs/                # Internal docs
```

**Issues with current structure:**

- `core/` is monolithic — mixes schemas, domain logic, I/O, and orchestration
- No separation between apps and libraries
- Adapters bundled together instead of per-integration
- No shared tooling configuration packages
- Scripts scattered rather than organised

## Target State

```
anvil/
├── apps/                    # Deployable applications
│   ├── anvil-cli/          # CLI (from cli/)
│   ├── anvil-api/          # API gateway (new)
│   ├── anvil-ui/           # Web UI (new)
│   ├── website/            # Marketing (new)
│   ├── docs-site/          # Public docs (new)
│   └── e2e/                # E2E test suites
│
├── packages/
│   ├── anvil/              # Core domain (from core/)
│   │   ├── contracts/      # Schemas, events, types
│   │   ├── ports/          # Interfaces only
│   │   ├── core/           # Pure domain logic
│   │   ├── runtime/        # Orchestration
│   │   ├── policy/         # OPA wrappers
│   │   └── sdk/            # Client SDK
│   │
│   ├── edda-stack/         # Memory/proposal system
│   │   ├── contracts/
│   │   ├── ports/
│   │   ├── ember/
│   │   ├── edda/
│   │   └── testing/
│   │
│   ├── adapters/           # Per-integration
│   │   ├── adapter-github/
│   │   ├── adapter-opencode/
│   │   └── adapter-claude-code/
│   │
│   ├── platform/           # Infrastructure
│   │   ├── config/
│   │   ├── storage/
│   │   ├── telemetry/
│   │   ├── auth/
│   │   ├── crypto/
│   │   └── http/
│   │
│   ├── shared/             # Utilities
│   │   ├── util/
│   │   ├── testing/
│   │   └── brand/
│   │
│   └── tooling/            # Build config
│       ├── eslint-config/
│       ├── tsconfig/
│       └── release/
│
├── tools/                  # Generators and scripts
│   ├── generators/
│   ├── scripts/
│   └── docker/
│
└── docs/                   # Internal docs
    ├── architecture/
    ├── decisions/
    ├── runbooks/
    └── security/
```

## Boundaries

### In Scope

- Moving existing packages to new locations
- Splitting `core/` into layered packages
- Splitting `adapters/` into per-integration packages
- Creating Nx project configurations
- Updating import paths (automated via codemod)
- Updating workspace configuration
- Creating package scaffolds for new locations

### Out of Scope

- Implementing new apps (API, UI, website, docs-site)
- Implementing edda-stack functionality
- Changing package functionality (move only)
- Rewriting tests (path updates only)

## Success Criteria

- [ ] All packages in target locations
- [ ] All 2,168+ tests pass after migration
- [ ] Build completes successfully
- [ ] No circular dependencies introduced
- [ ] Import paths use new `@anvil/*` aliases
- [ ] Nx project graph shows correct dependencies

## Interfaces

### Package Naming

| Current | Target |
|---------|--------|
| `@anvil/core` | `@anvil/contracts`, `@anvil/ports`, `@anvil/core`, `@anvil/runtime`, `@anvil/policy`, `@anvil/sdk` |
| `@anvil/cli` | `@anvil/cli` (unchanged, new location) |
| `@anvil/adapters` | `@anvil/adapter-github`, `@anvil/adapter-opencode`, etc. |

### Dependency Direction

```
apps → runtime → core → ports → contracts
         ↓
      platform
         ↓
      shared
```

No package may depend on a package above it in this hierarchy.

## Tasks

### Phase 1: Tooling Setup

#### MONO-001: Create Nx generators for package scaffolding

- **Intent:** Enable consistent package creation in target structure
- **Expected Outcome:** `nx g @anvil/tools:package` creates correctly configured package
- **Validation:** `nx g @anvil/tools:package --name=test-pkg --dry-run` shows expected output
- **Status:** Ready
- **Priority:** high

#### MONO-002: Create import path codemod

- **Intent:** Automate import path updates across codebase
- **Expected Outcome:** Codemod updates all `@anvil/*` imports to new paths
- **Validation:** `pnpm codemod:imports --dry-run` shows expected changes
- **Status:** Ready
- **Priority:** high

#### MONO-003: Create shared tooling packages

- **Intent:** Centralise ESLint and TypeScript configurations
- **Expected Outcome:** `@anvil/eslint-config` and `@anvil/tsconfig` packages exist
- **Validation:** `pnpm build` succeeds with shared configs
- **Status:** Ready
- **Priority:** medium

### Phase 2: Core Split

#### MONO-004: Extract contracts package from core

- **Intent:** Isolate schemas, types, and events with no dependencies
- **Expected Outcome:** `packages/anvil/contracts/` contains all Zod schemas
- **Validation:** `nx test @anvil/contracts` passes
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-001, MONO-002

#### MONO-005: Extract ports package from core

- **Intent:** Define interfaces without implementations
- **Expected Outcome:** `packages/anvil/ports/` contains interface definitions
- **Validation:** `nx build @anvil/ports` succeeds
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-004

#### MONO-006: Extract pure domain logic to core package

- **Intent:** Isolate business logic from I/O concerns
- **Expected Outcome:** `packages/anvil/core/` contains pure domain functions
- **Validation:** `nx test @anvil/core` passes with no I/O mocks
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-005

#### MONO-007: Extract runtime package

- **Intent:** Isolate orchestration and execution logic
- **Expected Outcome:** `packages/anvil/runtime/` contains runner, executor
- **Validation:** `nx test @anvil/runtime` passes
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-006

#### MONO-008: Extract policy package

- **Intent:** Isolate OPA/Rego integration
- **Expected Outcome:** `packages/anvil/policy/` contains OPA wrappers
- **Validation:** `nx test @anvil/policy` passes
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-006

### Phase 3: Platform Extract

#### MONO-009: Extract config package

- **Intent:** Centralise configuration loading and validation
- **Expected Outcome:** `packages/platform/config/` handles all config
- **Validation:** `nx test @anvil/config` passes
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-004

#### MONO-010: Extract storage package

- **Intent:** Abstract file system and cache operations
- **Expected Outcome:** `packages/platform/storage/` handles persistence
- **Validation:** `nx test @anvil/storage` passes
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-005

#### MONO-011: Extract crypto package

- **Intent:** Centralise hashing, signing, verification
- **Expected Outcome:** `packages/platform/crypto/` handles crypto ops
- **Validation:** `nx test @anvil/crypto` passes
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-004

### Phase 4: Adapters Split

#### MONO-012: Split adapters into per-integration packages

- **Intent:** Enable independent versioning and optional installation
- **Expected Outcome:** Separate packages for each adapter type
- **Validation:** `nx test @anvil/adapter-*` passes for all adapters
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-007

### Phase 5: App Migration

#### MONO-013: Move CLI to apps/anvil-cli

- **Intent:** Establish apps/ convention for deployables
- **Expected Outcome:** CLI builds and runs from new location
- **Validation:** `nx build @anvil/cli && anvil --version` works
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-007, MONO-008

#### MONO-014: Reorganise E2E tests

- **Intent:** Structure E2E tests by application
- **Expected Outcome:** `apps/e2e/cli-e2e/` contains CLI E2E tests
- **Validation:** `nx e2e cli-e2e` passes
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-013

#### MONO-015: Move scripts to tools/

- **Intent:** Centralise build and utility scripts
- **Expected Outcome:** `tools/scripts/` contains all build scripts
- **Validation:** Build scripts execute from new location
- **Status:** Ready
- **Priority:** low
- **Dependencies:** None

### Phase 6: Validation

#### MONO-016: Full test suite validation

- **Intent:** Ensure migration didn't break functionality
- **Expected Outcome:** All tests pass with new structure
- **Validation:** `pnpm test -- --run` shows 2,168+ tests passing
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-001 through MONO-015

#### MONO-017: Dependency graph validation

- **Intent:** Ensure no circular dependencies or incorrect edges
- **Expected Outcome:** Nx graph shows clean dependency tree
- **Validation:** `nx graph` shows expected structure
- **Status:** Ready
- **Priority:** high
- **Dependencies:** MONO-016

#### MONO-018: Documentation update

- **Intent:** Update docs to reflect new structure
- **Expected Outcome:** README, CONTRIBUTING, and guides updated
- **Validation:** Docs reference correct paths
- **Status:** Ready
- **Priority:** medium
- **Dependencies:** MONO-016

## Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Import path breakage | High | Medium | Automated codemod with dry-run |
| Circular dependencies | High | Low | Nx boundary constraints |
| Build order issues | Medium | Medium | Explicit Nx dependencies |
| Test failures | Medium | Medium | Run tests after each phase |
| CI pipeline breaks | Medium | Low | Feature branch for migration |

## Rollback Plan

Each phase is committed separately. Rollback by reverting commits in reverse
order. The codemod is reversible.

## Open Questions

- [ ] Should we version packages independently or keep unified versioning?
- [ ] Should adapters be published to npm or remain internal?
- [ ] Should we use Nx release or changesets for versioning?

## References

- [Impact Assessment](../docs/planning/monorepo-cleanup-impact-assessment.md)
- [Monorepo Structure](../docs/MONOREPO_STRUCTURE.md)
- [ADR: Nx Workspace](./decisions/007-nx-workspace.md) (to be created)
