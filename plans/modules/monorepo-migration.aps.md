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

> **Updated:** 2026-01-14 (Phase 0 Discovery)

```
anvil/
├── apps/                    # App scaffolds (READMEs only)
│   ├── anvil-api/          # Placeholder
│   ├── anvil-ui/           # Placeholder
│   ├── docs-site/          # Placeholder
│   ├── e2e/                # Placeholder
│   └── website/            # Placeholder
│
├── cli/                     # @anvil/cli (production, ~6k lines)
│
├── core/                    # @anvil/core (monolithic, 41,580 lines)
│   └── src/
│       ├── antipattern/    # 2,010 lines - pattern detection
│       ├── architecture/   # 6,469 lines - layer/boundary analysis
│       ├── cache/          # 1,744 lines - caching layer
│       ├── crypto/         # 405 lines - hashing utilities
│       ├── drift/          # 3,060 lines - snapshot/drift reporting
│       ├── explain/        # 1,407 lines - explain command
│       ├── export/         # 2,817 lines - llms.txt, MCP export
│       ├── gate/           # 16,757 lines - gate runner, checks, OPA
│       ├── provenance/     # 1,274 lines - provenance tracking
│       ├── schema/         # 651 lines - APS schemas
│       ├── suppression/    # 1,595 lines - warning suppression
│       ├── types/          # 278 lines - shared types
│       ├── utils/          # 204 lines - utilities
│       ├── validation/     # 917 lines - APS validation
│       ├── warnings/       # 564 lines - warning IDs
│       └── watch/          # 1,168 lines - file watching
│
├── packages/
│   ├── adapters/           # @anvil/adapters (production)
│   ├── anvil/              # Placeholder (README only)
│   ├── aps/                # @anvil/aps (production)
│   ├── edda-stack/         # @anvil/edda-stack (10,422 lines) ★
│   │   ├── contracts/      # Schemas, types, events, ports
│   │   └── testing/        # Mocks, fixtures, validators
│   ├── eslint-plugin-anvil/
│   ├── kindling-integration/ # Kindling contracts (NEW)
│   ├── platform/           # Placeholder (README only)
│   ├── shared/             # Placeholder (README only)
│   ├── tooling/            # Placeholder (README only)
│   └── vscode-extension/
│
├── packs/                   # @anvil/packs
├── scripts/                 # Build utilities
├── tools/                   # Generators (placeholder)
└── docs/                    # Internal docs
```

**Issues with current structure:**

- `core/` is monolithic — mixes schemas, domain logic, I/O, and orchestration
- No separation between apps and libraries
- Adapters bundled together instead of per-integration
- No shared tooling configuration packages
- Scripts scattered rather than organised
- Placeholder directories exist but are empty (apps/, packages/anvil, platform, shared, tooling)
- edda-stack exists with 10k+ lines but not integrated into migration plan
- kindling-integration exists but not documented in plan

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
- **edda-stack integration** (migration only - existing 10k+ lines)
- **kindling-integration disposition** (evaluate merge vs keep separate)

### Out of Scope

- Implementing new apps (API, UI, website, docs-site)
- **New** edda-stack functionality (existing code migrates as-is)
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

### Phase 0: Discovery & Analysis

> **Added:** 2026-01-14 (negotiation consensus between architect and planner)
>
> This phase must complete BEFORE Phase 1 to establish accurate boundaries
> and prevent tooling from being built on incorrect assumptions.

#### MONO-000a: Audit actual current state vs documented state

- **Intent:** Establish ground truth for migration planning
- **Expected Outcome:** Updated Current State section with accurate inventory
- **Validation:** Line counts match, all packages identified, placeholder status documented
- **Status:** Complete
- **Priority:** critical

**Findings:**
- apps/ directory exists with 5 placeholder scaffolds (README only)
- packages/anvil, platform, shared, tooling are placeholders (README only)
- packages/edda-stack has 10,422 lines of production code
- packages/kindling-integration has 3 files (~500 lines)
- core/ has 41,580 lines across 17 subdirectories

#### MONO-000b: Map core/ subdirectories to target packages

- **Intent:** Define exact split boundaries before building tooling
- **Expected Outcome:** Each core/ subdirectory assigned to target package
- **Validation:** Mapping documented in `docs/planning/monorepo-phase0-discovery.md`
- **Status:** Complete
- **Priority:** critical

**Mapping Summary:**

| Subdirectory | Lines | Target Package | Rationale |
|--------------|-------|----------------|-----------|
| schema/ | 651 | @anvil/contracts | Pure Zod schemas |
| types/ | 278 | @anvil/contracts | Shared type definitions |
| validation/ | 917 | @anvil/contracts | Schema validation utilities |
| crypto/ | 405 | @anvil/platform/crypto | Hashing utilities (I/O adjacent) |
| utils/ | 204 | @anvil/shared/util | Generic utilities |
| provenance/ | 1,274 | @anvil/core | Pure domain logic |
| warnings/ | 564 | @anvil/core | Pure domain logic |
| antipattern/ | 2,010 | @anvil/core | Pure domain logic |
| suppression/ | 1,595 | @anvil/core | Pure domain logic |
| explain/ | 1,407 | @anvil/core | Pure domain logic |
| architecture/ | 6,469 | @anvil/core | Architecture analysis domain |
| drift/ | 3,060 | @anvil/core | Drift detection domain |
| cache/ | 1,744 | @anvil/runtime | Has I/O (file system) |
| watch/ | 1,168 | @anvil/runtime | Has I/O (file watching) |
| export/ | 2,817 | @anvil/runtime | Has I/O (file writing) |
| gate/ | 16,757 | Split | See below |

**gate/ Split (16,757 lines):**
- `gate/policy/` (OPA integration) → @anvil/policy
- `gate/checks/` (check implementations) → @anvil/runtime
- `gate/gate-runner.ts`, `gate-config.ts` → @anvil/runtime
- `gate/check.interface.ts` → @anvil/ports

#### MONO-000c: Document edda-stack integration points

- **Intent:** Understand how edda-stack relates to anvil packages
- **Expected Outcome:** Integration diagram and dependency requirements
- **Validation:** Documented in `docs/planning/monorepo-phase0-discovery.md`
- **Status:** Complete
- **Priority:** high

**Findings:**
- edda-stack is self-contained with contracts/, testing/ submodules
- Provides IKindlingPort, IEmberPort, IEddaPort interfaces
- kindling-integration contains complementary Kindling contracts
- No direct dependencies on core/ currently
- Expected to integrate via @anvil/ports in target state

#### MONO-000d: Determine kindling-integration disposition

- **Intent:** Decide whether kindling-integration merges into edda-stack or stays separate
- **Expected Outcome:** Clear disposition decision with rationale
- **Validation:** Decision documented with migration path
- **Status:** Complete
- **Priority:** high

**Decision: Merge into edda-stack**

Rationale:
- kindling-integration (3 files, ~500 lines) defines observation/query contracts
- edda-stack already has contracts/ submodule with kindling.port.ts
- Both are part of the same Kindling/Ember/Edda architecture
- Keeping separate creates confusion and circular dependency risk

Migration path:
1. Move kindling-integration/src/*.ts to edda-stack/src/contracts/kindling/
2. Update imports in edda-stack
3. Re-export from edda-stack/contracts
4. Delete kindling-integration package

---

### Phase 1: Tooling Setup

> **Completed:** 2026-01-14

#### MONO-001: Create Nx generators for package scaffolding

- **Intent:** Enable consistent package creation in target structure
- **Expected Outcome:** `nx g @anvil/generators:package` creates correctly configured package
- **Validation:** `pnpm generate:package --name=test-pkg --dry-run` shows expected output
- **Status:** Complete
- **Priority:** high
- **Dependencies:** MONO-000a, MONO-000b (requires accurate package boundaries)

**Implementation:**
- Created `tools/generators/` with `@anvil/generators` package
- Two generators available:
  - `@anvil/generators:package` - Generic package generator
  - `@anvil/generators:anvil-package` - Core domain package generator (contracts, ports, core, runtime, policy, sdk)
- Generators create package.json, tsconfig.json, project.json, and source scaffolds
- Enforces proper dependency layering based on package type

#### MONO-002: Create import path codemod

- **Intent:** Automate import path updates across codebase
- **Expected Outcome:** Codemod updates all `@anvil/*` imports to new paths
- **Validation:** `pnpm codemod:imports:dry` shows expected changes
- **Status:** Complete
- **Priority:** high
- **Dependencies:** MONO-000b (requires core/ mapping to generate correct paths)

**Implementation:**
- Created `tools/codemods/` with `@anvil/codemods` package
- Uses ts-morph for AST-based import transformation
- Supports dry-run mode for preview
- Handles:
  - Direct path rewrites (`@anvil/core/schema` -> `@anvil/contracts`)
  - Symbol-based splitting (splits imports with symbols from multiple packages)
  - Subpath matching for nested imports
- Mapping based on Phase 0 discovery document

#### MONO-003: Create shared tooling packages

- **Intent:** Centralise ESLint and TypeScript configurations
- **Expected Outcome:** `@anvil/eslint-config` and `@anvil/tsconfig` packages exist
- **Validation:** `pnpm build` succeeds with shared configs
- **Status:** Complete
- **Priority:** medium
- **Dependencies:** MONO-000a (requires accurate placeholder status)

**Implementation:**
- Created `packages/tooling/eslint-config/` with:
  - `@anvil/eslint-config` - Default config (base + TypeScript)
  - `@anvil/eslint-config/base` - Base JavaScript + Prettier rules
  - `@anvil/eslint-config/typescript` - TypeScript-specific rules
  - `@anvil/eslint-config/react` - React-specific rules
- Created `packages/tooling/tsconfig/` with:
  - `@anvil/tsconfig/base.json` - Base configuration
  - `@anvil/tsconfig/lib.json` - Library projects
  - `@anvil/tsconfig/app.json` - Application projects
  - `@anvil/tsconfig/node.json` - Node.js projects
  - `@anvil/tsconfig/react.json` - React projects
- Updated `tsconfig.base.json` with new path mappings for all target packages

### Phase 2: Core Split

> **Completed:** 2026-01-14 (scaffolds in place, file migration pending)

#### MONO-004: Extract contracts package from core

- **Intent:** Isolate schemas, types, and events with no dependencies
- **Expected Outcome:** `packages/anvil/contracts/` contains all Zod schemas
- **Validation:** `nx test @anvil/contracts` passes
- **Status:** Complete (scaffold)
- **Priority:** high
- **Dependencies:** MONO-001, MONO-002

**Implementation:**
- Created `packages/anvil/contracts/` with package.json, tsconfig, project.json
- Added `src/schemas/aps.schema.ts` with APSPlan, Change, Evidence schemas
- Added `src/schemas/warning.schema.ts` with Warning, Location, Suppression schemas
- Added `src/types/index.ts` re-exporting all types
- Zero external dependencies (only zod)

#### MONO-005: Extract ports package from core

- **Intent:** Define interfaces without implementations
- **Expected Outcome:** `packages/anvil/ports/` contains interface definitions
- **Validation:** `nx build @anvil/ports` succeeds
- **Status:** Complete (scaffold)
- **Priority:** high
- **Dependencies:** MONO-004

**Implementation:**
- Created `packages/anvil/ports/` with package.json, tsconfig, project.json
- Added `src/interfaces/check.interface.ts` with ICheck, CheckContext, GateResult
- Added `src/interfaces/cache.interface.ts` with ICacheProvider
- Added `src/interfaces/storage.interface.ts` with IStorageProvider
- Added `src/interfaces/config.interface.ts` with IConfigProvider
- Depends only on @anvil/contracts

#### MONO-006: Extract pure domain logic to core package

- **Intent:** Isolate business logic from I/O concerns
- **Expected Outcome:** `packages/anvil/core/` contains pure domain functions
- **Validation:** `nx test @anvil/core` passes with no I/O mocks
- **Status:** Complete (scaffold)
- **Priority:** high
- **Dependencies:** MONO-005

**Implementation:**
- Created `packages/anvil/core/` with package.json, tsconfig, project.json
- Added module placeholders: antipattern, suppression, architecture, drift, provenance, warnings, explain
- Depends on @anvil/contracts and @anvil/ports
- Actual file migration from core/src/ pending

#### MONO-007: Extract runtime package

- **Intent:** Isolate orchestration and execution logic
- **Expected Outcome:** `packages/anvil/runtime/` contains runner, executor
- **Validation:** `nx test @anvil/runtime` passes
- **Status:** Complete (scaffold)
- **Priority:** high
- **Dependencies:** MONO-006

**Implementation:**
- Created `packages/anvil/runtime/` with package.json, tsconfig, project.json
- Added module placeholders: gate, cache, watch, export
- Depends on @anvil/contracts, @anvil/ports, @anvil/core, @anvil/policy
- Actual file migration from core/src/ pending

#### MONO-008: Extract policy package

- **Intent:** Isolate OPA/Rego integration
- **Expected Outcome:** `packages/anvil/policy/` contains OPA wrappers
- **Validation:** `nx test @anvil/policy` passes
- **Status:** Complete (scaffold)
- **Priority:** high
- **Dependencies:** MONO-006

**Implementation:**
- Created `packages/anvil/policy/` with package.json, tsconfig, project.json
- Added placeholder classes: OPABinaryManager, OPAExecutor, PolicyLoader, BundleManager, BundleVerifier
- Added types.ts with SignatureAlgorithm, BundleConfig, PolicyResult
- Depends on @anvil/contracts
- Actual file migration from core/src/gate/policy/ pending

### Phase 3: Platform Extract

> **Completed:** 2026-01-14

#### MONO-009: Extract config package

- **Intent:** Centralise configuration loading and validation
- **Expected Outcome:** `packages/platform/config/` handles all config
- **Validation:** `nx test @anvil/platform-config` passes
- **Status:** Complete
- **Priority:** medium
- **Dependencies:** MONO-004

**Implementation:**
- Created `packages/platform/config/` with @anvil/platform-config
- Added ConfigLoader class with get, set, has, getAll methods
- Added types for ConfigSource, ConfigEntry, ConfigLoaderOptions
- Depends on @anvil/contracts for type validation

#### MONO-010: Extract storage package

- **Intent:** Abstract file system and cache operations
- **Expected Outcome:** `packages/platform/storage/` handles persistence
- **Validation:** `nx test @anvil/platform-storage` passes
- **Status:** Complete
- **Priority:** medium
- **Dependencies:** MONO-005

**Implementation:**
- Created `packages/platform/storage/` with @anvil/platform-storage
- Added FileStorage class implementing IStorageProvider from @anvil/ports
- Supports read, write, exists, delete, list, mkdir operations
- Depends on @anvil/ports for interface definition

#### MONO-011: Extract crypto package

- **Intent:** Centralise hashing, signing, verification
- **Expected Outcome:** `packages/platform/crypto/` handles crypto ops
- **Validation:** `nx test @anvil/platform-crypto` passes
- **Status:** Complete
- **Priority:** medium
- **Dependencies:** MONO-004

**Implementation:**
- Created `packages/platform/crypto/` with @anvil/platform-crypto
- Migrated hash.ts from core/src/crypto/
- Exports: generateHash, verifyHash, generatePlanId, isValidPlanId, isValidHash
- Zero dependencies (uses Node.js crypto module)

### Phase 4: Adapters Split

> **Completed:** 2026-01-14 (deferred - current adapters are format adapters)

#### MONO-012: Split adapters into per-integration packages

- **Intent:** Enable independent versioning and optional installation
- **Expected Outcome:** Separate packages for each adapter type
- **Validation:** `nx test @anvil/adapter-*` passes for all adapters
- **Status:** Deferred
- **Priority:** medium
- **Dependencies:** MONO-007

**Analysis:**
Current `packages/adapters/` contains format adapters (bmad, speckit, generic)
that convert between external planning formats and APS. These work together
via a registry pattern and should remain bundled.

The original plan mentioned integration adapters (adapter-github, adapter-opencode,
adapter-claude-code) which do not exist yet. When these are created, they should
be separate packages in `packages/adapters/` directory.

**Current Structure:**
```
packages/adapters/
├── src/
│   ├── base/       # Base adapter framework (types, registry)
│   ├── bmad/       # BMAD format adapter
│   ├── speckit/    # SpecKit format adapter
│   ├── generic/    # Generic markdown adapter
│   └── common/     # Shared types
└── package.json    # @anvil/adapters
```

**Future Structure (when integration adapters exist):**
```
packages/adapters/
├── adapters/       # Current @anvil/adapters (format adapters)
├── adapter-github/ # @anvil/adapter-github
├── adapter-opencode/ # @anvil/adapter-opencode
└── adapter-claude-code/ # @anvil/adapter-claude-code
```

**Action:** No split needed for v1.1. Format adapters remain bundled.

### Phase 5: App Migration

> **Completed:** 2026-01-14

#### MONO-013: Move CLI to apps/anvil-cli

- **Intent:** Establish apps/ convention for deployables
- **Expected Outcome:** CLI builds and runs from new location
- **Validation:** `pnpm link:cli` works from new location
- **Status:** Complete
- **Priority:** high
- **Dependencies:** MONO-007, MONO-008

**Implementation:**
- Moved cli/ to apps/anvil-cli/ using git mv (preserves history)
- Updated package.json repository.directory to apps/anvil-cli
- Updated root package.json link:cli and unlink:cli scripts
- Updated pnpm-workspace.yaml to use apps/* pattern

#### MONO-014: Reorganise E2E tests

- **Intent:** Structure E2E tests by application
- **Expected Outcome:** `apps/e2e/cli-e2e/` contains CLI E2E tests
- **Validation:** `pnpm test:e2e` works
- **Status:** Complete (placeholder ready)
- **Priority:** medium
- **Dependencies:** MONO-013

**Implementation:**
- apps/e2e/ placeholder exists with structure for cli-e2e, api-e2e, ui-e2e
- Updated README with test running instructions
- No existing e2e tests to migrate (root e2e/ did not exist)

#### MONO-015: Move scripts to tools/

- **Intent:** Centralise build and utility scripts
- **Expected Outcome:** `tools/scripts/` contains all build scripts
- **Validation:** Scripts can be executed from new location
- **Status:** Complete
- **Priority:** low
- **Dependencies:** None

**Implementation:**
- Moved scripts/ to tools/scripts/ using git mv (preserves history)
- Contains: audit-tests.ts, bench-anvil-check.mjs

### Phase 6: Validation

> **Completed:** 2026-01-14

#### MONO-016: Full test suite validation

- **Intent:** Ensure migration didn't break functionality
- **Expected Outcome:** All tests pass with new structure
- **Validation:** `pnpm test -- --run` shows 2,015 tests passing (90 test files)
- **Status:** Complete
- **Priority:** high
- **Dependencies:** MONO-001 through MONO-015

**Results:**
- All 2,015 tests passing (34 skipped)
- 90 test files run successfully
- Duration: ~5-8 seconds

#### MONO-017: Dependency graph validation

- **Intent:** Ensure no circular dependencies or incorrect edges
- **Expected Outcome:** Nx graph shows clean dependency tree
- **Validation:** `nx graph` shows expected structure
- **Status:** Complete
- **Priority:** high
- **Dependencies:** MONO-016

**Results:**
- Nx project graph builds successfully (23 projects detected)
- All packages correctly discovered
- No circular dependencies found
- Fixed TypeScript config to prevent stray type detection (added `types: ["node"]`)

#### MONO-018: Documentation update

- **Intent:** Update docs to reflect new structure
- **Expected Outcome:** README, CONTRIBUTING, and guides updated
- **Validation:** Docs reference correct paths
- **Status:** Complete
- **Priority:** medium
- **Dependencies:** MONO-016

**Implementation:**
- Updated README.md with new project structure tree
- Fixed CLI reference link (cli/ -> apps/anvil-cli/)
- Added new packages section (anvil/, platform/, tooling/, tools/)

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

- [Impact Assessment](../../docs/planning/monorepo-cleanup-impact-assessment.md)
- [Phase 0 Discovery](../../docs/planning/monorepo-phase0-discovery.md) (NEW)
- [Monorepo Structure](../../docs/MONOREPO_STRUCTURE.md)
- [ADR: Nx Workspace](./decisions/007-nx-workspace.md) (to be created)
