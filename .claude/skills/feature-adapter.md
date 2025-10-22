# feature-adapter

Implement a new feature in an adapter following established patterns. Guides
through adapter-specific development.

## Parameters

- **adapter_name**: Target adapter (e.g., "speckit", "bmad")
- **feature_description**: What to implement (e.g., "metadata extraction",
  "validation rules", "export formatting")

## Tasks

1. **Search for Similar Implementations**
   - Look in other adapters for similar features:
     - If implementing for `bmad`, check how `speckit` does it
     - Search for pattern across all adapters: `Grep` pattern in
       `packages/adapters/src/`
   - Read 2-3 implementations in parallel
   - Identify common approach

2. **Identify Relevant Files to Modify** Adapter structure:

   ```
   packages/adapters/src/<adapter-name>/
   ├── import.ts       # Import adapter (format → APS)
   ├── export.ts       # Export adapter (APS → format)
   ├── parser.ts       # Core parsing logic
   ├── parsers/        # Specialized parsers
   │   ├── metadata.ts
   │   ├── requirements.ts
   │   └── ...
   └── __tests__/      # Test files
   ```

   Determine which files need changes:
   - New parsing: Add to `parser.ts` or `parsers/`
   - Import changes: Modify `import.ts` or `import-v2.ts`
   - Export changes: Modify `export.ts`
   - New adapter: Create all files

3. **Add/Update Type Definitions**
   - Add types to reflect new feature:
     ```typescript
     // parser.ts
     export interface ParsedMetadata {
       title: string;
       newFeature?: string; // Add new field
     }
     ```
   - Update APS mapping types if needed
   - Ensure types flow through entire pipeline

4. **Implement Parsing Logic** Following existing patterns:

   ```typescript
   // parsers/metadata.ts
   export function extractNewFeature(content: string): string | undefined {
     // Use same pattern as other extractors
     const match = content.match(/^new-feature:\s*(.+)$/m);
     return match?.[1]?.trim();
   }
   ```

   Patterns to follow:
   - Use regex with named groups or clear captures
   - Handle multiline content if needed
   - Return undefined for optional fields
   - Throw descriptive errors for required fields

5. **Update Import Adapter** Integrate new feature into import flow:

   ```typescript
   // import-v2.ts
   async convert(source: FormatSource): Promise<APS> {
     const parsed = parseFormat(source.content);

     return {
       metadata: {
         title: parsed.metadata.title,
         newFeature: parsed.metadata.newFeature,  // Add here
         // ...
       },
       // ...
     };
   }
   ```

6. **Update Export Adapter** (if applicable) Ensure round-trip compatibility:

   ```typescript
   // export.ts
   async convert(aps: APS): Promise<string> {
     let output = '';

     output += `title: ${aps.metadata.title}\n`;
     if (aps.metadata.newFeature) {
       output += `new-feature: ${aps.metadata.newFeature}\n`;
     }

     return output;
   }
   ```

7. **Add Comprehensive Tests**
   - Test parsing in isolation:

     ```typescript
     describe('extractNewFeature', () => {
       it('should extract new feature from content', () => {
         const content = 'new-feature: test value\n';
         expect(extractNewFeature(content)).toBe('test value');
       });

       it('should return undefined when not present', () => {
         expect(extractNewFeature('')).toBeUndefined();
       });
     });
     ```

   - Test import adapter:

     ```typescript
     it('should import new feature field', async () => {
       const source = createMockFormatSource({
         content: specWithNewFeature,
       });
       const aps = await adapter.convert(source);
       expect(aps.metadata.newFeature).toBe('expected value');
     });
     ```

   - Test export adapter (round-trip):
     ```typescript
     it('should export and re-import new feature', async () => {
       const original: APS = {
         metadata: { newFeature: 'test' },
         // ...
       };
       const exported = await exportAdapter.convert(original);
       const reimported = await importAdapter.convert({
         fileName: 'test',
         content: exported,
       });
       expect(reimported.metadata.newFeature).toBe('test');
     });
     ```

8. **Update Adapter README** Document the new feature:

   ```markdown
   ## Features

   ### Metadata Extraction

   - Title, version, description
   - **New Feature**: Extracts XYZ from format (added v1.2)

   ### Example

   Input format: \`\`\` new-feature: value here \`\`\`

   Maps to APS: \`\`\`json { "metadata": { "newFeature": "value here" } } \`\`\`
   ```

9. **Run Adapter-Specific Tests**

   ```bash
   # Run all adapter tests
   npx nx test adapters -- src/${adapter_name}

   # Run with coverage
   pnpm test -- packages/adapters/src/${adapter_name} --coverage

   # Check specific test file
   pnpm test -- parser.test.ts --watch
   ```

10. **Verify Integration** Test end-to-end through CLI if applicable:

    ```bash
    # Test import
    pnpm anvil validate examples/${adapter_name}/spec.md

    # Test export (when implemented)
    pnpm anvil export examples/aps.json --format ${adapter_name}
    ```

## Adapter Feature Patterns

### Metadata Fields

```typescript
// Always optional unless explicitly required by spec
interface Metadata {
  required: string;
  optional?: string;
}

// Extract with safe defaults
const optional = extractField(content) ?? defaultValue;
```

### Requirement Parsing

```typescript
// Requirements are typically lists
function parseRequirements(section: string): Requirement[] {
  const lines = section.split('\n');
  return lines
    .filter((line) => line.trim().startsWith('-'))
    .map((line) => parseRequirementLine(line));
}
```

### User Story Parsing

```typescript
// User stories may be multiline
function extractUserStory(content: string): string | undefined {
  const match = content.match(
    /As a (.+?)\nI want (.+?)\nSo that (.+?)(?=\n\n|$)/s
  );
  if (!match) return undefined;
  return `As a ${match[1]}\nI want ${match[2]}\nSo that ${match[3]}`;
}
```

### Section Extraction

```typescript
// Sections delimited by headers
function extractSection(content: string, header: string): string {
  const regex = new RegExp(`^## ${header}\\s*\\n([\\s\\S]*?)(?=^## |$)`, 'gm');
  const match = regex.exec(content);
  return match?.[1]?.trim() ?? '';
}
```

### Validation Rules

```typescript
// Validate during parsing, throw descriptive errors
function parseWithValidation(content: string): Parsed {
  const title = extractTitle(content);
  if (!title) {
    throw new Error('Missing required field: title');
  }

  const requirements = parseRequirements(content);
  if (requirements.length === 0) {
    throw new Error('Must have at least one requirement');
  }

  return { title, requirements };
}
```

## Testing Checklist

- ✅ Parse valid input correctly
- ✅ Handle missing optional fields
- ✅ Reject invalid required fields
- ✅ Handle edge cases (empty, special chars)
- ✅ Maintain backward compatibility
- ✅ Round-trip (export → import) produces equivalent result
- ✅ Error messages are clear and actionable
- ✅ No regression in existing tests

## Example: Adding Tags Feature to SpecKit

```typescript
// 1. Update type
interface SpecKitMetadata {
  title: string;
  tags?: string[]; // NEW
}

// 2. Add parser (parsers/metadata.ts)
export function extractTags(content: string): string[] {
  const match = content.match(/^tags:\s*(.+)$/m);
  if (!match) return [];
  return match[1].split(',').map((t) => t.trim());
}

// 3. Update import (import-v2.ts)
const tags = extractTags(source.content);
// Add to metadata: { tags }

// 4. Update export (export.ts)
if (aps.metadata.tags?.length) {
  output += `tags: ${aps.metadata.tags.join(', ')}\n`;
}

// 5. Add tests
describe('extractTags', () => {
  it('should extract comma-separated tags', () => {
    expect(extractTags('tags: foo, bar')).toEqual(['foo', 'bar']);
  });

  it('should return empty array when missing', () => {
    expect(extractTags('')).toEqual([]);
  });
});

// ✅ Tests pass, feature complete
```

## Anvil Adapter Specifics

- Base types: `packages/adapters/src/base/types.ts`
- Test utilities: `packages/adapters/src/base/testing.ts`
- APS schema: `@anvil/core/schema`
- Adapters must implement `FormatAdapter` interface
- Use `createMockFormatSource()` in tests
- Registry auto-detects adapters by convention
- Current adapters: SpecKit (stable), BMAD (planned)
