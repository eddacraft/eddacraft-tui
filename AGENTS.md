# AGENTS.md - AI Agent Instructions for EddaCraft/Anvil

## Build & Test Commands

```bash
# Essential commands
pnpm build                    # Build all packages (required before tests)
pnpm test                     # Run all unit tests (Vitest)
pnpm test -- path/to/file.test.ts    # Run single test file
pnpm test -- -t "test name"  # Run tests matching pattern
pnpm lint                     # oxlint + nx lint + markdownlint
pnpm format                   # oxfmt formatting
pnpm typecheck                # TypeScript strict mode

# Package-specific
nx test <pkg> --testNamePattern="pattern"  # Test pattern in package
pnpm -F <package> run <script>             # Run package script
```

### Running Single Tests

```bash
# Single test file
pnpm test -- packages/anvil/runtime/src/gate/gate-runner.test.ts

# Test by name pattern
pnpm test -- -t "should handle failing checks"

# Package-specific test
nx test @eddacraft/anvil-core --testNamePattern="validator"
```

## Code Style Guidelines

### Imports

```typescript
// ✅ ESM with .js extensions required
import { CheckContext } from '../types/gate.types.js';

// ✅ Node built-ins use node: protocol
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// ❌ Wrong - missing .js or node: prefix
import { foo } from './utils';
import { readFileSync } from 'fs';
```

### Naming Conventions

- **Files**: kebab-case (`gate-runner.ts`, `format-detection.ts`)
- **Variables/Functions**: camelCase
- **Types/Interfaces**: PascalCase
- **Unused vars**: Prefix with `_` (`_unusedArg`, `_context`)
- **Tests**: Co-located as `.test.ts` or `.spec.ts`

### Types & Validation

```typescript
// Zod-first schemas (ADR-0001)
export const ChangeSchema = z.object({
  type: ChangeTypeSchema,
  path: z.string().describe('File path'),
});
export type Change = z.infer<typeof ChangeSchema>;
```

### Error Handling

- Never use empty catch blocks `catch(e) {}`
- Always handle or properly log errors
- Use typed errors where possible

### Formatting

- **Quotes**: Single quotes
- **Trailing commas**: es5
- **Line length**: 100 chars (80 for markdown)
- **Line endings**: LF (Unix)

## UK English Required

Use UK spelling: organised, recognised, colour, behaviour, optimise, centre,
analyse, initialise, serialise.

## Anti-Patterns (Forbidden)

| Pattern                     | Reason                     |
| --------------------------- | -------------------------- |
| `as any`, `@ts-ignore`      | Type safety non-negotiable |
| Empty catch blocks          | Silently swallows errors   |
| Missing `.js` extensions    | ESM runtime failure        |
| `console.log` in production | Use proper logging         |
| `cd dir && command`         | Use workdir parameter      |

## Planning & Specs

**Before implementing ANY feature:**

1. Read `plans/aps-rules.md` first
2. Check `plans/index.aps.md` for existing tasks
3. Create `.aps.md` files in `plans/modules/` for new features

Key rules:

- Specs describe intent, not implementation
- Tasks authorise execution with outcome + validation
- Steps are checkpoints (max 12 words, observable state)

## Key Directories

| Location                  | Purpose                    |
| ------------------------- | -------------------------- |
| `packages/anvil/core/`    | Schema, validation, crypto |
| `packages/anvil/runtime/` | Gate checks, execution     |
| `packages/adapters/`      | Format converters          |
| `packages/aps/`           | APS parser & validator     |
| `apps/anvil-cli/`         | CLI with TUI (Ink)         |

## Before Committing

All changes must pass:

```bash
pnpm build && pnpm test && pnpm lint && pnpm format && pnpm typecheck
```

CI rejects PRs that fail any check.

## Environment

- **Node.js**: >=22.13.0
- **pnpm**: >=10.20.0
- **TypeScript**: Strict mode enabled

## Gotchas

1. **Build before test**: `pnpm build` required for cross-package imports
2. **After schema changes**: Run `pnpm -F core run generate:schema` then
   `update-golden-hashes`
3. **ESM only**: This project uses `"type": "module"`
