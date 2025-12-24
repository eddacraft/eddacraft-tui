# Anvil - AI Agent Instructions

> **Deterministic development automation platform that makes AI-generated code
> changes safe for production**

Anvil validates and executes AI-generated code changes through the **Anvil Plan
Specification (APS)** – a hash-stable, internal interchange format that enables
deterministic validation and governance.

## Build & Test Commands

```bash
# Essential commands
pnpm build                  # Build all packages (REQUIRED before cross-package testing)
pnpm test                   # All unit tests (Vitest)
pnpm test:coverage          # Tests with coverage reports
pnpm test:e2e               # Playwright E2E tests
pnpm lint                   # ESLint + markdownlint with auto-fix
pnpm typecheck              # TypeScript strict mode validation
pnpm format                 # Format with Prettier

# Single test patterns
nx test core --testNamePattern="validator"      # Run tests matching pattern
nx test adapters --testNamePattern="BMAD"       # Test specific adapter
nx test core --testNamePattern="secret"         # Test specific feature

# Package-specific
nx build core               # Build specific package
nx test cli                 # Test specific package
pnpm -F core run generate:schema              # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes         # Update golden test hashes

# CLI development
pnpm link:cli               # Build and link CLI globally ('anvil' command)
pnpm unlink:cli             # Unlink CLI when done
```

## Code Style Guidelines

### Language & Spelling

**Use UK English** throughout:

- organised, recognised, colour, behaviour, optimise, centre, analyse
- initialise (not initialize), serialise (not serialize)
- Applies to: documentation, comments, variable names, user-facing text

### TypeScript Conventions

**ESM with .js extensions** (NodeNext module resolution):

```typescript
// ✅ Correct - always use .js extension
import { CheckContext } from '../types/gate.types.js';
import { ChangeSchema } from '@anvil/core/schema/aps.schema.js';

// ❌ Wrong - will fail at runtime
import { foo } from './utils';
```

**Zod-first schemas** (see ADR-0001):

```typescript
// Define schema with Zod
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File or resource path'),
});

// Export inferred TypeScript type
export type Change = z.infer<typeof ChangeSchema>;
```

**Path aliases** for cross-package imports:

- `@anvil/core` → `core/src/index.ts`
- `@anvil/adapters` → `packages/adapters/src/index.ts`

### Formatting Rules

- **Prettier**: Single quotes, trailing commas (es5), semicolons, 100 char width
- **Markdown**: 80 char width, prose wrap always
- **ESLint**: Warn on `any`, prefer unused `_` prefix, console only for errors
- **Line endings**: LF (Unix-style)

### Naming Conventions

- **Unused variables**: Prefix with `_` (e.g., `_unusedArg`, `_context`)
- **Files**: kebab-case (e.g., `gate-runner.ts`, `format-detection.ts`)
- **Tests**: Co-located with source as `.test.ts` or `.spec.ts`
- **Fixtures**: Store in `__fixtures__/` directories

### Error Handling

- Use `BaseCheck` pattern for gate checks with `createSuccess()/createFailure()`
- Return structured `GateResult` objects with clear messages
- Prefer early returns and explicit error types
- Never use empty catch blocks

### Type Safety

- TypeScript strict mode enabled - all type errors must be resolved
- Never suppress errors with `as any`, `@ts-ignore`, or `@ts-expect-error`
- Use `z.infer<typeof Schema>` for types derived from Zod schemas

## Project Structure

```
anvil/
├── core/                     # @anvil/core - APS schema, validation, gate runner
│   └── src/
│       ├── schema/           # aps.schema.ts - Zod schema (v0.1.0)
│       ├── crypto/           # SHA-256 deterministic hashing
│       ├── gate/             # Quality gate checks
│       │   └── checks/       # ESLint, coverage, secrets, dependencies
│       ├── provenance/       # Audit trail and evidence collection
│       └── validation/       # APS validator with rich error messages
├── cli/                      # @anvil/cli - Command-line interface
│   └── src/
│       ├── commands/         # validate, gate, plan, init, export, watch
│       ├── services/         # format detection, plan loading
│       └── types/            # TypeScript type definitions
├── packages/
│   ├── adapters/             # @anvil/adapters - Format conversion
│   │   └── src/
│   │       ├── base/         # FormatAdapter interface, AdapterRegistry
│   │       ├── speckit/      # GitHub SpecKit adapter
│   │       ├── bmad/         # BMAD PRD/architecture adapter
│   │       └── generic/      # Generic markdown adapter
│   └── vscode-extension/     # VS Code extension
├── docs/                     # Documentation
│   ├── planning/             # ROADMAP.md, TODO.md
│   └── adr/                  # Architecture decision records
└── e2e/                      # Playwright E2E tests
```

## Critical Gotchas

### 1. Build Before Test

TypeScript project references require explicit builds before cross-package
imports work:

```bash
pnpm build        # Build all packages first
pnpm test         # Now tests can import from other packages
```

### 2. After Schema Changes

When modifying Zod schemas in `core/src/schema/`:

```bash
pnpm -F core run generate:schema        # Regenerate JSON schema
pnpm -F core run update-golden-hashes   # Update golden test hashes
pnpm test                               # Verify all tests pass
```

### 3. ESM Import Extensions

All imports MUST use `.js` extensions, even for TypeScript files:

```typescript
// This is required by NodeNext module resolution
import { foo } from './utils.js'; // ✅ Correct
import { foo } from './utils'; // ❌ Will fail at runtime
```

### 4. Pre-existing Issues

When you encounter pre-existing issues (failing tests, lint errors, type errors)
**not related to your current task**:

1. **Flag the issue** - Report what you found with specific details
2. **Ask for permission** - Do not silently fix or ignore
3. **Separate concerns** - If approved, fix in a separate commit

Example: "I noticed a pre-existing failing test in `memory-cache.test.ts`
(unrelated to my changes). Would you like me to investigate?"

## Testing Patterns

- **Location**: Co-locate tests with source files
- **Naming**: `.test.ts` or `.spec.ts` extension
- **Fixtures**: Store in `__fixtures__/` directories
- **Golden files**: `core/src/__fixtures__/golden-plans/` for hash verification
- **Coverage**: Exclude `index.ts` and test files

```bash
# Run specific test file
nx test core --testFile="validator.test.ts"

# Run tests matching pattern
nx test core --testNamePattern="should validate"

# Run with coverage
pnpm test:coverage
```

## Key Architecture Concepts

### APS (Anvil Plan Specification)

The internal hash-stable format enabling deterministic validation. Users work in
external formats; Anvil translates to APS internally.

```
External Format (SpecKit/BMAD) → Adapter → APS (internal) → Gate → Execute
```

### Pipeline Flow

1. **Parse**: External format → APS (via adapter)
2. **Validate**: Schema validation + hash generation
3. **Gate**: Quality checks (lint, test, coverage, secrets)
4. **Execute**: Apply changes with snapshots (rollback capability)
5. **Evidence**: Immutable audit trail

### Gate Checks

Located in `core/src/gate/checks/`:

- `architecture.check.ts` - Architecture validation
- `coverage.check.ts` - Code coverage thresholds
- `dependency.check.ts` - Vulnerability scanning
- `eslint.check.ts` - Code quality
- `policy.check.ts` - OPA/Rego policy evaluation
- `secret.check.ts` - Pattern + entropy detection

## Nx Monorepo

Always prefer running tasks through Nx:

```bash
nx build <package>          # Build specific package
nx test <package>           # Test specific package
nx affected -t test         # Test only changed packages
nx graph                    # Visualise dependency graph
```

**Package names**: Use folder names (`core`, `cli`, `adapters`), not npm names
(`@anvil/core`).

## Environment Requirements

- **Node.js**: >=20.0.0
- **pnpm**: >=10.20.0

## CLI Commands (after `pnpm link:cli`)

```bash
anvil init                   # Initialise Anvil in a project
anvil validate <plan>        # Validate plan against APS schema
anvil gate <plan>            # Run quality gates
anvil export <plan> --to <format>  # Convert between formats
anvil watch                  # Watch for plan changes
anvil plan                   # Plan management
```

## Documentation References

- `docs/ARCHITECTURE.md` - System design and principles
- `docs/adr/` - Architecture decision records
- `docs/planning/TODO.md` - Task tracking
- `core/API.md` - APS Core API reference
- `packages/adapters/README.md` - Adapter framework guide
