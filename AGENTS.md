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

# Build Commands

```bash
pnpm build              # Build all packages (use before testing cross-package changes)
nx build <package>      # Build specific package (core, cli, adapters)
pnpm test               # Run all tests
nx test <package>       # Test specific package
pnpm test:coverage      # Run tests with coverage
pnpm test:e2e           # Run Playwright e2e tests
pnpm lint               # Run ESLint + markdownlint with auto-fix
pnpm typecheck          # TypeScript strict mode validation
```

**Single test**: `npx nx test core --testNamePattern="validator"`

# Code Style Guidelines

## Language & Conventions

- **UK English** spelling: organised, recognised, colour, behaviour, optimise,
  etc.
- **TypeScript strict mode** - all type errors must be resolved
- **ESM modules** - use `.js` extensions in imports even for `.ts` files
- **Modern syntax** - ES2022 + Node.js (??, optional chaining, top-level await)

## Imports & Structure

```typescript
// ✅ Correct - use .js extensions
import { CheckContext } from '../types/gate.types.js';

// ✅ Use path aliases for cross-package imports
import { ChangeSchema } from '@anvil/core/schema/aps.schema.js';
```

## Schema & Validation

**Always use Zod for schemas**:

```typescript
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File or resource path'),
});
export type Change = z.infer<typeof ChangeSchema>;
```

## Formatting Rules

- **Prettier**: Single quotes, trailing commas es5, 100 char width
- **ESLint**: Warn on `any`, prefer unused `_` prefix, console only for errors
- **Semicolons**: Required
- **Arrow functions**: Always with parentheses for parameters

## Error Handling

- Use `BaseCheck` pattern for gate checks with `createSuccess()/createFailure()`
- Return structured `GateResult` objects with clear messages
- Prefer early returns and explicit error types

## Testing Patterns

- **Location**: Co-locate tests (`.test.ts` or `.spec.ts`) with source
- **Vitest**: Use globals, happy-dom environment
- **Fixtures**: Store in `__fixtures__/` directories
- **Coverage**: Exclude `index.ts` and test files from coverage calculations
