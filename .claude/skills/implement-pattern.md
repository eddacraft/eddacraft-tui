# implement-pattern

Find and follow existing patterns when implementing new code. Ensures
consistency with the codebase.

## Parameters

- **pattern_type**: Type of pattern to follow (e.g., "parser", "adapter",
  "validator", "CLI command", "test suite")
- **target**: What you're implementing (brief description)

## Tasks

1. **Find Pattern Examples**
   - Search for existing implementations of this pattern type:
     - **Adapter**: `packages/adapters/src/*/import.ts`, `export.ts`
     - **Parser**: `packages/adapters/src/*/parser.ts`
     - **Validator**: `packages/core/src/validation/*.ts`
     - **CLI Command**: `cli/src/commands/*.ts`
     - **Test**: Look for `*.test.ts` files in the same package
   - Read 2-3 representative examples in parallel

2. **Extract Common Structure**
   - Identify consistent patterns across examples:
     - File structure and organisation
     - Function/class naming conventions
     - Parameter patterns
     - Return types
     - Error handling approach
   - Note any variations and why they exist

3. **Identify Required Types and Interfaces**
   - List interfaces that must be implemented
   - Show type definitions that should be used
   - Highlight generics or type parameters
   - Note any discriminated unions or branded types

4. **Show Template/Boilerplate**
   - Create a skeleton showing the pattern structure
   - Include:
     - Imports (organised by internal/external)
     - Type definitions
     - Main implementation structure
     - Error handling
     - Exports
   - Add inline comments explaining each section

5. **Implement Following Pattern**
   - Write the actual implementation using the pattern
   - Match:
     - Naming conventions exactly
     - Import style and organisation
     - Error handling patterns
     - Documentation style (JSDoc format)
   - Keep similar levels of abstraction

6. **Add Tests Matching Pattern**
   - Find test files for the pattern examples
   - Identify test structure:
     - `describe()` block organization
     - Test naming convention (`it('should ...')`)
     - Setup/teardown patterns
     - Mock/fixture usage
   - Write tests following the same style

7. **Validation**
   - Run type check: `npx nx typecheck <package>`
   - Run lint: `pnpm lint:check`
   - Run tests: `pnpm test -- <test-file>`
   - Verify consistency with examples

## Pattern-Specific Guidelines

### Adapter Pattern

```typescript
// Structure: packages/adapters/src/<format>/
// Required files: import.ts, export.ts, parser.ts

export class MyFormatImportAdapter implements FormatAdapter {
  name = 'my-format-import';

  canHandle(source: FormatSource): boolean {
    // Detection logic matching format signatures
  }

  async convert(source: FormatSource): Promise<APS> {
    // Parse, transform, validate
  }
}
```

### Parser Pattern

```typescript
// Pure functions, well-tested
export function parseMyFormat(content: string): ParsedData {
  // 1. Extract sections
  // 2. Parse each section
  // 3. Validate structure
  // 4. Return typed result
}
```

### CLI Command Pattern

```typescript
// Located in: cli/src/commands/<name>.ts
export const myCommand = {
  name: 'my-command',
  description: 'Brief description',
  options: [
    /* yargs options */
  ],
  handler: async (argv) => {
    // 1. Load plan/config
    // 2. Execute operation
    // 3. Display results
  },
};
```

### Validator Pattern

```typescript
// Uses Zod schemas
export const mySchema = z.object({
  field: z.string(),
  // ...
});

export type MyType = z.infer<typeof mySchema>;

export function validateMyData(data: unknown): MyType {
  return mySchema.parse(data);
}
```

## Example Usage

When called with `pattern_type: "adapter"` and `target: "BMAD format import"`:

```typescript
// Found patterns: speckit/import.ts, speckit/import-v2.ts

// Template:
import type { FormatAdapter, FormatSource } from '../base/types';
import type { APS } from '@anvil/core/schema';
import { parseBMAD } from './parser';

export class BMADImportAdapter implements FormatAdapter {
  readonly name = 'bmad-import';
  readonly description =
    'Import BMAD (Blueprint Model-Based Artifact Description) format';

  canHandle(source: FormatSource): boolean {
    // Check for BMAD format markers
    return (
      source.content.includes('---') &&
      source.content.includes('artifact:') &&
      source.content.includes('blueprint:')
    );
  }

  async convert(source: FormatSource): Promise<APS> {
    const parsed = parseBMAD(source.content);

    // Transform to APS
    const aps: APS = {
      metadata: {
        title: parsed.title,
        version: parsed.version,
        // ...
      },
      // ...
    };

    return aps;
  }
}
```

## Anvil Project Conventions

- **Imports**: External first, then internal, alphabetically
- **Naming**: camelCase for functions, PascalCase for classes/types
- **Error handling**: Use try/catch for async, throw descriptive errors
- **Tests**: 100% coverage expected for core, 80%+ for adapters
- **Documentation**: JSDoc for public APIs, inline comments for complex logic
- **File organization**: Group by feature, co-locate tests
