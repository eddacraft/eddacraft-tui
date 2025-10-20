# Copilot Instructions for Anvil

## Language Convention

**Use UK English** for all documentation, comments, variable names, and
user-facing text throughout the codebase.

## Project Overview

Anvil is a deterministic development automation platform that transforms AI and
human intent into validated, auditable, and reversible changes. It solves a
critical problem: **making AI-generated code changes safe for production** by
enforcing quality gates, maintaining complete audit trails, and ensuring every
change is reversible.

### What Makes Anvil Different

- **Format Agnostic**: Work with SpecKit, BMAD, or native APS – your choice
- **Comprehensive Gates**: ESLint, Vitest, coverage thresholds, secret scanning,
  OPA policies
- **Production Safety**: Snapshot-based rollback with transactional execution
- **Full Provenance**: Track who, what, when, why for every change
- **GitHub Native**: First-class PR integration that blocks unsafe merges

### Core Design Principles

1. **Interoperability First**: Users keep their existing planning formats. Anvil
   works with them through pluggable adapters rather than forcing format
   adoption.
2. **Determinism**: Same input → same output, always. Hash-stable plans enable
   reproducible validation, tamper detection, and confident reviews.
3. **Safety by Default**: Every change creates a snapshot before application.
   Rollback is a first-class operation, not an afterthought.
4. **Transparency**: Immutable, append-only evidence bundles provide complete
   audit trails for compliance and debugging.
5. **Composability**: Parse → Validate → Execute are independent, composable
   stages. Each component has clear interfaces and can be used in library mode.

## Architecture & Pipeline Flow

Anvil operates as a validation and execution pipeline built around the **Anvil
Plan Specification (APS)** – a hash-stable, internal interchange format that
enables deterministic validation and governance:

```
User Formats (SpecKit, BMAD, etc.)
    ↓
Adapters (parse to APS)
    ↓
Quality Gates (lint, test, coverage, secrets, policies)
    ↓
Safe Execution (apply with snapshots, rollback capability)
    ↓
Immutable Audit Trail
```

**Key Insight**: APS is the moat. It's hash-stable (SHA-256), schema-validated
(Zod), and enables deterministic validation. Users work in their preferred
format; Anvil translates everything to APS internally.

## Project Structure

```
anvil/
├── core/              # APS schema, validation, hashing, gate runner
│   └── src/
│       ├── schema/    # aps.schema.ts - Zod schema (v0.1.0)
│       ├── crypto/    # SHA-256 deterministic hashing
│       ├── gate/      # Quality gate checks (ESLint, coverage, secrets)
│       └── validation/ # APS validator with rich error messages
├── packages/adapters/ # Format conversion (SpecKit ✅, BMAD planned)
│   └── src/
│       ├── base/      # FormatAdapter interface, AdapterRegistry
│       ├── speckit/   # GitHub spec-kit adapter (2.5k LOC, 51 tests)
│       └── bmad/      # [Future] Business architecture docs
├── cli/               # Commander.js CLI (gate, plan, validate commands)
└── packs/             # [Future] Feature bundles (flags, telemetry)
```

## Development Workflow

### Building and Testing

```bash
pnpm install           # Install dependencies (requires pnpm 10.17.1+)
pnpm build             # Build all packages (respects TypeScript project references)
pnpm test              # Vitest unit tests
pnpm test:coverage     # With coverage reports
pnpm typecheck         # TypeScript strict mode validation
```

**Critical**: Always run `pnpm build` before testing cross-package changes.
TypeScript project references require explicit builds.

### Nx Commands

```bash
npx nx build <package>        # Build specific package
npx nx test <package>         # Test specific package
npx nx graph                  # Visualize dependency graph
npx nx affected -t test       # Test only affected projects
```

**Project Names**: Use folder names directly (`core`, `cli`, `adapters`), not
package names (`@anvil/core`).

### Running CLI in Development

```bash
cd cli
pnpm build                    # Build CLI
node dist/index.js gate <plan-id>  # Run gate command
node dist/index.js plan <file>     # Run plan command
```

**Note**: CLI uses ES modules (`"type": "module"`). All imports require `.js`
extensions even for `.ts` files.

## Code Conventions

### TypeScript

- **Strict mode enabled**: All type errors must be resolved
- **ES2022 + Node.js**: Use modern syntax (`??`, optional chaining, top-level
  await)
- **Module system**: `"module": "nodenext"` - use `.js` extensions in imports
- **Path aliases**: `@anvil/adapters` maps to `packages/adapters/src/index.ts`

### Schema & Validation

**Always use Zod for schema definition**, not manual JSON schemas:

```typescript
// ✅ Correct - Define with Zod
export const ChangeSchema = z.object({
  type: z.enum(['file_create', 'file_update', ...]),
  path: z.string(),
  description: z.string(),
});

// ✅ Export TypeScript types
export type Change = z.infer<typeof ChangeSchema>;
```

**APS Schema Version**: Current version is `0.1.0`. When updating:

1. Update version in `core/src/schema/aps.schema.ts`
2. Regenerate JSON schema: `pnpm -F core run generate:schema`
3. Update golden test hashes: `pnpm -F core run update-golden-hashes`

### Adapters

**Creating new adapters**: Follow SpecKit as reference
(`packages/adapters/src/speckit/`):

1. Implement `FormatAdapter` interface from `base/types.ts`
2. Register with `AdapterRegistry` (singleton)
3. Support auto-detection via `canHandle()` with confidence scoring
4. Ensure round-trip fidelity (parse → APS → serialize preserves intent)
5. Add comprehensive tests (see `__tests__/speckit-import-v2.test.ts`)

**Key Files**:

- `base/types.ts` - Core interfaces (`FormatAdapter`, `ParseResult`,
  `DetectionResult`)
- `base/registry.ts` - Singleton registry for adapter discovery
- `speckit/import-adapter-v2.ts` - Complete implementation example

### Gate Checks

**Adding new gate checks**: Implement `Check` interface from
`core/src/gate/check.interface.ts`:

```typescript
export class MyCheck implements Check {
  name = 'my-check';
  description = 'What this check validates';

  async run(context: CheckContext): Promise<GateResult> {
    // Return: { check, passed, message, score?, details?, error? }
  }
}
```

**Register in GateRunner**: Add to `registerDefaultChecks()` in
`gate-runner.ts`.

## Testing Patterns

### Unit Tests (Vitest)

- **Location**: Co-located with source (`.test.ts` or `.spec.ts`)
- **Fixtures**: Use `__fixtures__/` for test data
- **Golden Files**: `golden-plans/` contain hash-verified reference plans

```typescript
describe('ComponentName', () => {
  it('should do specific thing', () => {
    // Arrange
    const input = createTestInput();
    // Act
    const result = component.process(input);
    // Assert
    expect(result).toMatchSnapshot(); // Or specific assertions
  });
});
```

**Run specific tests**: `npx nx test core --testNamePattern="validator"`

### Integration Tests

See `cli/src/__tests__/cli-aps-integration.test.ts` for examples of
cross-package testing.

## Common Gotchas

1. **Missing `.js` extensions**: ESM requires explicit extensions even for
   TypeScript

   ```typescript
   // ❌ Wrong
   import { foo } from './utils';

   // ✅ Correct
   import { foo } from './utils.js';
   ```

2. **Build before testing**: Cross-package imports require built artifacts

   ```bash
   pnpm build        # Must run before tests can import from other packages
   pnpm test         # Now tests can import built packages
   ```

3. **TypeScript project references**: Each package has `tsconfig.json` with
   `references` array. Add new packages to dependent package configs.

4. **Path aliases**: Defined in `tsconfig.base.json` under
   `compilerOptions.paths`. Update when adding new packages.

5. **APS schema changes**: Always regenerate JSON schema and update golden
   hashes after schema modifications.

## Current Implementation Status

- ✅ **APS Core** (schema, validation, hashing): 80% complete
- ✅ **Quality Gates** (lint, test, coverage, secrets): 90% complete
- 🚧 **Adapters** (SpecKit ✅ complete, BMAD in progress): 50% complete
- 🚧 **CLI** with format auto-detection: In progress (30%)
- 📋 **Apply/Rollback** with audit trail: Planned
- 📋 **GitHub Action** integration: Planned

**Next Sprint Focus**: CLI integration with SpecKit adapter, format
auto-detection, BMAD adapter.

See [TODO.md](../TODO.md) for detailed task tracking and [PLAN.md](../PLAN.md)
for strategic roadmap.

### Strategic Vision (Three Acts)

- **Act 1**: Development automation for engineers (current focus – MVP)
- **Act 2**: Document validation for analysts and consultants (12-18 months)
- **Act 3**: Horizontal platform for governance across all knowledge work (24+
  months)

## Key Documentation

- `ARCHITECTURE.md` - System design, principles, data flows (1,575 lines)
- `CLAUDE.md` - Project overview and commands
- `core/API.md` - APS Core API reference
- `packages/adapters/ADAPTER_WORKFLOW_GUIDE.md` - Creating new adapters
- `docs/GIT_WORKTREE_WORKFLOW.md` - Parallel plan development workflow

## References

When updating core schema or validation logic:

1. Verify all tests pass: `pnpm test`
2. Check TypeScript compilation: `pnpm typecheck`
3. Update golden hashes if needed: `pnpm -F core run update-golden-hashes`
4. Regenerate JSON schema if schema changed: `pnpm -F core run generate:schema`
5. Update documentation in `core/API.md` and `core/EXAMPLES.md`

**Example Files to Study**:

- `core/src/schema/aps.schema.ts` - Zod schema patterns
- `packages/adapters/src/speckit/import-adapter-v2.ts` - Complete adapter
  implementation
- `core/src/gate/gate-runner.ts` - Check registration and execution
- `cli/src/commands/gate.ts` - CLI command structure with ora spinners
