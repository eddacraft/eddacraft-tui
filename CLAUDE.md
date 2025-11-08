# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Essential Build & Test Commands

```bash
# Build all packages (CRITICAL: always build before testing cross-package changes)
pnpm build

# Run tests
pnpm test                    # All unit tests
pnpm test:coverage           # With coverage reports
pnpm typecheck               # TypeScript validation

# Code quality
pnpm lint                    # Lint and auto-fix
pnpm format                  # Format with Prettier

# Package-specific operations
npx nx build core            # Build specific package
npx nx test adapters         # Test specific package
pnpm -F core run generate:schema          # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes     # Update golden test hashes after schema changes
```

## Critical Architecture Concepts

### APS (Anvil Plan Specification) - The Moat

APS is the **internal** hash-stable format that enables deterministic
validation. Users never see it—they work in their own formats (SpecKit, BMAD,
etc.).

```
External Format (SpecKit/BMAD) → Adapter → APS (internal) → Gate → Execute
```

**Key insight**: Adapters are the wedge. Users keep existing workflows; Anvil
just makes them safer.

### Pipeline Flow

```
1. Parse: External format → APS (via adapter)
2. Validate: Schema validation + hash generation
3. Gate: Quality checks (lint, test, coverage, secrets)
4. Execute: Apply changes with snapshots (rollback capability)
5. Evidence: Immutable audit trail
```

### Core Components

**APS Core** (`core/`):

- `schema/aps.schema.ts` - Zod schema (v0.1.0), source of truth
- `crypto/hash.ts` - SHA-256 deterministic hashing
- `gate/` - Quality gate checks (ESLint, Vitest, coverage, secrets)
- `validation/` - APS validator

**Adapters** (`packages/adapters/`):

- `base/types.ts` - FormatAdapter interface
- `base/registry.ts` - Singleton registry for auto-detection
- `speckit/` - GitHub spec-kit adapter (2.5k LOC, 51 tests)
- `bmad/` - BMAD PRD/architecture adapter (~800 LOC)

**CLI** (`cli/`):

- `commands/validate.ts` - Validate plans
- `commands/gate.ts` - Run quality gates
- `commands/export.ts` - Format conversion
- `services/format-detection.ts` - Auto-detect format
- `services/plan-loader.ts` - Load plans in any format

## TypeScript Conventions

**ESM with .js extensions** (NodeNext module resolution):

```typescript
// ✅ Correct
import { foo } from './utils.js';

// ❌ Wrong - will fail at runtime
import { foo } from './utils';
```

**Zod-first schemas**:

```typescript
// Define schema with Zod
export const MySchema = z.object({
  field: z.string(),
});

// Export inferred TypeScript type
export type MyType = z.infer<typeof MySchema>;
```

**TypeScript project references**: Each package references dependencies in
`tsconfig.json`. This requires explicit builds before cross-package imports
work:

```bash
pnpm build    # Build all packages first
pnpm test     # Now tests can import from other packages
```

## Common Workflows

### Adding a New Format Adapter

Reference: `packages/adapters/src/bmad/` (most recent implementation)

1. Implement `FormatAdapter` interface from `base/types.ts`:
   - `detect()` - Format detection with confidence scoring
   - `parse()` - External format → APS
   - `serialize()` - APS → External format
   - `validate()` - Fast validation without full conversion
   - `canImport()` / `canExport()` - Format support checks

2. Register with `AdapterRegistry` in adapter module

3. Add comprehensive tests (target: 50+ tests like SpecKit)

4. Verify CLI integration:
   ```bash
   anvil validate <file>
   anvil gate <file>
   anvil export <file> --to aps
   ```

### Debugging Build Errors

**"Cannot find module" errors**:

```bash
pnpm build  # TypeScript project references require builds
```

**Tests failing after schema changes**:

```bash
pnpm -F core run update-golden-hashes
```

**Vitest config not in rootDir**:

- Ensure `rootDir: "."` in `tsconfig.spec.json`, not `"src"`

### After APS Schema Changes

1. Update `core/src/schema/aps.schema.ts` (Zod schema)
2. Update version if needed: `schema_version: z.literal('0.1.0')`
3. Regenerate JSON schema: `pnpm -F core run generate:schema`
4. Update golden hashes: `pnpm -F core run update-golden-hashes`
5. Run all tests: `pnpm test`

## Project Structure

```
anvil/
├── core/                 # APS schema, validation, hashing, gates
│   ├── src/schema/       # Zod schemas (source of truth)
│   ├── src/crypto/       # Deterministic hashing
│   ├── src/gate/         # Quality checks
│   └── src/validation/   # APS validation
├── packages/adapters/    # Format conversion (SpecKit, BMAD)
│   ├── src/base/         # FormatAdapter interface
│   ├── src/speckit/      # GitHub spec-kit adapter
│   └── src/bmad/         # BMAD adapter
├── cli/                  # Commander.js CLI
│   ├── src/commands/     # CLI commands
│   └── src/services/     # Format detection, plan loading
└── docs/                 # Documentation
    ├── ARCHITECTURE.md   # System design (1,575 lines)
    ├── planning/PLAN.md  # Strategic roadmap
    └── planning/TODO.md  # Task tracking
```

## Language Convention

**Always use UK English** for:

- Documentation and comments
- Variable names (`colour` not `color`, `initialise` not `initialize`)
- User-facing text

## Current Implementation Status (October 2025)

✅ **Complete**:

- APS Core (schema, validation, hashing) - 100%
- Adapter Framework (base types, registry) - 100%
- SpecKit Adapter (v1 + v2, 51 tests) - 100%
- BMAD Adapter (PRD/architecture format) - 100%
- CLI Integration (validate, gate, export) - 100%
- Gate v1 (lint, test, coverage, secrets) - 100%

🚧 **In Progress**:

- BMAD adapter unit tests (target: 50+ tests)
- SpecKit adapter migration to FormatAdapter interface

📋 **Planned**:

- Policy Engine (OPA/Rego)
- Apply/Rollback with snapshots
- GitHub Action integration

## Key Design Principles

1. **Interoperability First** - Support existing formats, don't force adoption
2. **Determinism** - Same input → same output, always (hash-stable)
3. **Safety by Default** - Snapshot before apply, rollback is first-class
4. **Transparency** - Immutable evidence bundles for audit trails
5. **Composability** - Parse → Validate → Execute are independent stages

## Testing Patterns

- **Co-locate tests**: `.test.ts` or `.spec.ts` alongside source
- **Fixtures**: Use `__fixtures__/` directories
- **Golden files**: `core/src/__fixtures__/golden-plans/` for hash verification
- **Integration tests**: `cli/src/__tests__/cli-aps-integration.test.ts`

Run specific tests:

```bash
npx nx test core --testNamePattern="validator"
npx nx test adapters --testNamePattern="BMAD"
```

## Documentation

**Architecture & Design**:

- `docs/ARCHITECTURE.md` - System design (1,575 lines)
- `docs/planning/PLAN.md` - Strategic vision (three acts)
- `docs/planning/TODO.md` - Task tracking with progress

**Package Documentation**:

- `core/API.md` - APS Core API reference
- `packages/adapters/README.md` - Adapter framework
- `packages/adapters/ADAPTER_WORKFLOW_GUIDE.md` - Creating adapters

**Development**:

- `README.md` - Single source of truth for setup and commands

## Nx Monorepo

This is an Nx workspace with TypeScript strict mode:

```bash
npx nx graph                   # Visualise dependencies
npx nx affected -t test        # Test only changed packages
npx nx affected -t build       # Build only changed packages
```

**Package names**: Use folder names (`core`, `cli`, `adapters`), not npm package
names (`@anvil/core`).

<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

### General Guidelines for working with Nx

- When running tasks (for example build, lint, test, e2e, etc.), always prefer
  running the task through `nx` (i.e. `nx run`, `nx run-many`, `nx affected`)
  instead of using the underlying tooling directly
- You have access to the Nx MCP server and its tools, use them to help the user
- When answering questions about the repository, use the `nx_workspace` tool
  first to gain an understanding of the workspace architecture where applicable.
- When working in individual projects, use the `nx_project_details` mcp tool to
  analyse and understand the specific project structure and dependencies
- For questions around nx configuration, best practices or if you're unsure, use
  the `nx_docs` tool to get relevant, up-to-date docs. Always use this instead
  of assuming things about nx configuration
- If the user needs help with an Nx configuration or project graph error, use
  the `nx_workspace` tool to get any errors

<!-- nx configuration end-->
