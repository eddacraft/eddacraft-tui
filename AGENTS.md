# Anvil - AI Agent Instructions

> **Deterministic development automation platform that makes AI-generated code
> changes safe for production**

Anvil validates and executes AI-generated code changes through the **Anvil Plan
Specification (APS)** – a hash-stable, internal interchange format that enables
deterministic validation and governance.

<!-- nx configuration start-->
<!-- Leave the start & end comments to automatically receive updates. -->

# General Guidelines for working with Nx

- When running tasks (for example build, lint, test, e2e, etc.), always prefer
  running the task through `nx` (i.e. `nx run`, `nx run-many`, `nx affected`)
  instead of using the underlying tooling directly
- You have access to the Nx MCP server and its tools, use them to help the user
- When answering questions about the repository, use the `nx_workspace` tool
  first to gain an understanding of the workspace architecture where applicable.
- When working in individual projects, use the `nx_project_details` mcp tool to
  analyze and understand the specific project structure and dependencies
- For questions around nx configuration, best practices or if you're unsure, use
  the `nx_docs` tool to get relevant, up-to-date docs. Always use this instead
  of assuming things about nx configuration
- If the user needs help with an Nx configuration or project graph error, use
  the `nx_workspace` tool to get any errors

<!-- nx configuration end-->

# Project Structure

```
anvil/
├── cli/                      # @anvil/cli - Command-line interface
│   └── src/
│       ├── commands/         # validate, gate, plan, init, export, watch
│       ├── services/         # format detection, plan loading, evidence
│       └── types/            # TypeScript type definitions
├── core/                     # @anvil/core - APS schema, validation, gate runner
│   └── src/
│       ├── cache/            # Caching providers
│       ├── crypto/           # SHA-256 deterministic hashing
│       ├── gate/             # Quality gate checks (ESLint, coverage, secrets)
│       │   ├── checks/       # Individual check implementations
│       │   └── policy/       # OPA policy engine integration
│       ├── provenance/       # Audit trail and evidence collection
│       ├── schema/           # APS Zod schema (v0.1.0)
│       ├── validation/       # APS validator with rich error messages
│       └── watch/            # File watching and orchestration
├── packages/
│   ├── adapters/             # @anvil/adapters - Format conversion
│   │   └── src/
│   │       ├── base/         # FormatAdapter interface, AdapterRegistry
│   │       ├── bmad/         # BMAD PRD/architecture adapter
│   │       ├── generic/      # Generic markdown plan adapter
│   │       └── speckit/      # GitHub SpecKit adapter
│   └── vscode-extension/     # anvil-vscode - VS Code extension
├── docs/                     # Project documentation
│   ├── planning/             # ROADMAP.md, TODO.md
│   └── guides/               # Development guides
├── e2e/                      # Playwright E2E tests
├── packs/                    # [Future] Feature bundles
└── ui/                       # [Future] UI components
```

# Build Commands

```bash
pnpm build              # Build all packages (use before testing cross-package changes)
nx build <package>      # Build specific package (core, cli, adapters)
pnpm test               # Run all tests with Vitest
nx test <package>       # Test specific package
pnpm test:coverage      # Run tests with coverage
pnpm test:e2e           # Run Playwright e2e tests
pnpm lint               # Run ESLint + markdownlint with auto-fix
pnpm typecheck          # TypeScript strict mode validation
pnpm format             # Format code with Prettier
```

**Single test**: `npx nx test core --testNamePattern="validator"`

**Package-specific**:

```bash
pnpm -F core run generate:schema        # Regenerate JSON schema from Zod
pnpm -F core run update-golden-hashes   # Update golden test hashes
pnpm link:cli                           # Link CLI globally for testing
```

# Code Style Guidelines

## Language & Conventions

- **UK English** spelling: organised, recognised, colour, behaviour, optimise,
  etc.
- **TypeScript strict mode** - all type errors must be resolved
- **ESM modules** - use `.js` extensions in imports even for `.ts` files
- **Modern syntax** - ES2022 + Node.js (??, optional chaining, top-level await)
- **Node.js**: >=20.0.0 required
- **pnpm**: >=10.20.0 required

## Imports & Structure

```typescript
// ✅ Correct - use .js extensions
import { CheckContext } from '../types/gate.types.js';

// ✅ Use path aliases for cross-package imports
import { ChangeSchema } from '@anvil/core/schema/aps.schema.js';
```

## Schema & Validation

**Always use Zod for schemas** (see ADR-0001):

```typescript
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File or resource path'),
});
export type Change = z.infer<typeof ChangeSchema>;
```

After modifying Zod schemas, regenerate JSON schema:

```bash
pnpm -F core run generate:schema
```

## Formatting Rules

- **Prettier**: Single quotes, trailing commas es5, 100 char width (80 for
  markdown)
- **ESLint**: Warn on `any`, prefer unused `_` prefix, console only for errors
- **Semicolons**: Required
- **Arrow functions**: Always with parentheses for parameters
- **Line endings**: LF (Unix-style)

## Error Handling

- Use `BaseCheck` pattern for gate checks with `createSuccess()/createFailure()`
- Return structured `GateResult` objects with clear messages
- Prefer early returns and explicit error types

## Testing Patterns

- **Location**: Co-locate tests (`.test.ts` or `.spec.ts`) with source
- **Vitest**: Use globals, happy-dom environment
- **Fixtures**: Store in `__fixtures__/` directories
- **Golden files**: `core/src/__fixtures__/golden-plans/` for hash-verified
  reference plans
- **Coverage**: Exclude `index.ts` and test files from coverage calculations

After schema changes that affect golden files:

```bash
pnpm -F core run update-golden-hashes
```

# Pre-existing Issues

When you encounter pre-existing issues in the codebase (failing tests, linting
errors, type errors, etc.) that are **not related to your current task**:

1. **Flag the issue** - Report what you found with specific details
2. **Ask for permission** - Do not silently fix or ignore pre-existing issues
3. **Separate concerns** - If approved, fix in a separate commit from your main
   work

Example:

> I noticed a pre-existing failing test in `memory-cache.test.ts` (unrelated to
> my changes). Would you like me to investigate and fix it?

This ensures:

- Clear attribution of changes
- No unexpected side effects
- Explicit approval for scope expansion

# Key Concepts

## Anvil Plan Specification (APS)

The internal interchange format. Hash-stable (SHA-256), schema-validated (Zod).
Users work in their preferred format (SpecKit, BMAD, generic); Anvil translates
to APS internally.

## Format Adapters

Located in `packages/adapters/src/`. Each adapter implements `FormatAdapter`
interface:

- **speckit**: GitHub SpecKit format (spec.md, plan.md, tasks.md)
- **bmad**: BMAD PRD/architecture format
- **generic**: Generic markdown plans

## Quality Gates

Located in `core/src/gate/checks/`. Available checks:

- `architecture.check.ts` - Architecture validation
- `coverage.check.ts` - Code coverage thresholds
- `dependency.check.ts` - Dependency analysis
- `secret.check.ts` - Secret scanning
- `eslint.check.ts` - Linting
- `policy.check.ts` - OPA/Rego policy evaluation

## CLI Commands

- `anvil validate <plan>` - Validate plan against APS schema
- `anvil gate <plan>` - Run quality gates on plan
- `anvil export <plan>` - Convert between formats
- `anvil init` - Initialise Anvil in a project
- `anvil watch` - Watch for plan changes
- `anvil plan` - Plan management
