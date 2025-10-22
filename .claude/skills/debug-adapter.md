# debug-adapter

Debug an adapter issue using systematic approach. Helps diagnose parsing,
import, or export problems.

## Parameters

- **adapter_name**: Name of the adapter with the issue (e.g., "speckit", "bmad")
- **issue_description**: Brief description of the problem
- **test_file**: (Optional) Path to failing test or example file that reproduces
  the issue

## Tasks

1. **Reproduce the Issue**
   - If `test_file` provided:
     - Run the specific test: `pnpm test -- <test_file>`
     - Capture error message and stack trace
   - If no test file:
     - Search for related tests: `**/*${adapter_name}*.test.ts`
     - Identify which test case should cover this scenario
     - Run all adapter tests to find failures

2. **Locate Relevant Code**
   - Find adapter implementation files:
     - `packages/adapters/src/${adapter_name}/import.ts`
     - `packages/adapters/src/${adapter_name}/export.ts`
     - `packages/adapters/src/${adapter_name}/parser.ts`
     - `packages/adapters/src/${adapter_name}/parsers/*.ts`
   - Read the files in parallel to understand structure

3. **Add Debugging Context**
   - Insert strategic console.log statements:
     ```typescript
     console.log('DEBUG: Input source:', JSON.stringify(source, null, 2));
     console.log('DEBUG: Parsed sections:', sections);
     console.log('DEBUG: Extracted metadata:', metadata);
     ```
   - Add logs at:
     - Function entry points
     - Before/after transformations
     - Error paths

4. **Run with Debugging**
   - Execute test in watch mode: `pnpm test -- <test_file> --watch`
   - Or run with node inspect if needed
   - Analyze debug output to trace data flow
   - Identify where actual behavior diverges from expected

5. **Identify Root Cause**
   - Common issues to check:
     - **Regex patterns**: Test regex against actual input
     - **String parsing**: Check for whitespace, newlines, special chars
     - **Type mismatches**: Verify expected vs actual types
     - **Missing edge cases**: Null, undefined, empty strings
     - **State management**: Check if parser maintains state incorrectly
   - Compare working vs failing inputs to find differences

6. **Implement Fix**
   - Make minimal change to fix the root cause
   - Follow existing patterns in the adapter
   - Consider if fix affects other code paths
   - Remove debug console.log statements

7. **Add Regression Test**
   - Create test case that would have caught this bug
   - Use the failing input as test fixture
   - Name test clearly: `it('should handle <specific case>', ...)`
   - Add to existing test suite in `__tests__/` directory

8. **Verify Fix**
   - Run all adapter tests: `npx nx test adapters -- src/${adapter_name}`
   - Ensure fix doesn't break other tests
   - Check test coverage is maintained or improved
   - Run typecheck: `npx nx typecheck adapters`

9. **Document the Issue** (if complex)
   - Add comment explaining the gotcha
   - Update adapter README if it affects usage
   - Note any limitations or edge cases

## Common Adapter Issues

### Regex Parsing Problems

```typescript
// Problem: Greedy regex captures too much
const bad = /title: (.+)/; // Captures everything including newlines

// Fix: Use non-greedy or specific patterns
const good = /title: (.+?)$/m; // Stops at line end
```

### Metadata Extraction

```typescript
// Problem: Optional fields cause undefined
const title = metadata.title.trim(); // Crashes if undefined

// Fix: Use optional chaining and defaults
const title = metadata.title?.trim() ?? 'Untitled';
```

### Multiline Content

```typescript
// Problem: Single-line regex misses multiline content
const pattern = /description: (.+)/;

// Fix: Use multiline flag and proper matching
const pattern = /description:\s*\n([\s\S]+?)(?=\n\w+:|$)/;
```

### Format Detection

```typescript
// Problem: canHandle() too permissive
canHandle(source: FormatSource): boolean {
  return source.content.includes('spec');  // Too broad!
}

// Fix: Check multiple distinctive markers
canHandle(source: FormatSource): boolean {
  return source.fileName?.includes('spec.md') &&
         source.content.includes('## Requirements') &&
         source.content.includes('## Design');
}
```

## Example Debugging Session

```
Issue: SpecKit V2 export adapter failing to preserve multiline user stories

1. Reproduce:
   $ pnpm test -- export.test.ts
   ✗ should preserve multiline user stories in output

2. Add debugging:
   console.log('DEBUG: User story:', requirement.userStory);
   console.log('DEBUG: Exported line:', exportedLine);

3. Found root cause:
   - User story has newlines: "As a user\nI want...\nSo that..."
   - Export template: `- ${userStory}` → breaks markdown
   - Need to indent continuation lines

4. Fix:
   const formatted = userStory.split('\n')
     .map((line, i) => i === 0 ? line : `  ${line}`)
     .join('\n');

5. Test passes, coverage maintained
```

## Anvil Project Specifics

- Adapters are in: `packages/adapters/src/<format>/`
- Test fixtures in: `packages/adapters/src/<format>/__tests__/fixtures/`
- Use `createMockFormatSource()` helper for tests
- Validate output against APS schema using `apsSchema.parse()`
- Current adapters: SpecKit (stable), BMAD (in progress)
