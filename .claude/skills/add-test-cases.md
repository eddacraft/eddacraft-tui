# add-test-cases

Add comprehensive test coverage for a module, focusing on untested code paths.

## Parameters

- **file_path**: Path to the file that needs test coverage
- **coverage_target**: (Optional) Target coverage percentage (default: 90%)

## Tasks

1. **Analyze Current Coverage**
   - Run coverage for the specific file:
     ```bash
     pnpm test -- <test-file> --coverage --reporter=json
     ```
   - Or get overall coverage: `pnpm test:coverage`
   - Parse coverage report to identify:
     - Uncovered lines
     - Uncovered branches
     - Uncovered functions
     - Current coverage percentage

2. **Review Existing Test Patterns**
   - Find test file for the module (or create path):
     - Same directory: `${file}.test.ts`
     - Tests directory: `__tests__/${basename}.test.ts`
   - Read existing tests to understand:
     - Test framework setup (Vitest, Jest, etc.)
     - Describe/it structure and naming
     - Mock patterns used
     - Fixture/test data approach

3. **Identify Untested Code Paths**
   - Read the source file
   - List functions/methods without tests
   - Identify branches not covered:
     - If/else statements
     - Switch cases
     - Ternary operators
     - Early returns
     - Error paths (try/catch)
   - Note edge cases not tested

4. **Plan Test Cases** For each untested path, identify:
   - **Happy path**: Normal, expected usage
   - **Edge cases**: Empty values, boundaries, max/min
   - **Error cases**: Invalid input, exceptions, timeouts
   - **Integration points**: External dependencies, APIs
   - **Side effects**: State changes, I/O operations

5. **Write Unit Tests** Following existing patterns:

   ```typescript
   describe('ModuleName', () => {
     describe('functionName', () => {
       it('should handle normal case', () => {
         const result = functionName(validInput);
         expect(result).toBe(expected);
       });

       it('should handle empty input', () => {
         const result = functionName('');
         expect(result).toBe(defaultValue);
       });

       it('should throw on invalid input', () => {
         expect(() => functionName(invalid)).toThrow();
       });

       it('should handle edge case: maximum value', () => {
         const result = functionName(Number.MAX_SAFE_INTEGER);
         expect(result).toBeDefined();
       });
     });
   });
   ```

6. **Add Edge Case Tests** Critical edge cases to cover:
   - **Null/undefined**: `null`, `undefined`, missing properties
   - **Empty**: `''`, `[]`, `{}`
   - **Boundaries**: 0, -1, MAX_VALUE, MIN_VALUE
   - **Special chars**: Unicode, newlines, quotes, escape sequences
   - **Large data**: Long strings, large arrays, deep nesting
   - **Concurrent**: Race conditions, async timing

7. **Add Error Scenario Tests**

   ```typescript
   describe('error handling', () => {
     it('should throw descriptive error for invalid format', () => {
       expect(() => parse(invalidContent)).toThrow(
         'Invalid format: missing required section'
       );
     });

     it('should handle network failure gracefully', async () => {
       mockFetch.mockRejectedValue(new Error('Network error'));
       await expect(fetchData()).rejects.toThrow('Network error');
     });

     it('should validate required fields', () => {
       const incomplete = { name: 'test' }; // missing required field
       expect(() => validate(incomplete)).toThrow('Missing required field: id');
     });
   });
   ```

8. **Run Tests with Coverage**
   - Execute tests: `pnpm test -- <test-file> --coverage`
   - Check coverage report
   - Verify target coverage is met
   - Identify any remaining gaps

9. **Verify Test Quality**
   - Tests are fast (< 100ms each for unit tests)
   - Tests are independent (can run in any order)
   - Tests are clear (obvious what they test)
   - Tests use appropriate assertions
   - No flaky tests (random failures)
   - Tests clean up after themselves

10. **Document Special Test Cases** Add comments for non-obvious tests:

    ```typescript
    // Test for issue #123: Parser fails on multiline metadata
    it('should parse multiline metadata values', () => {
      // ...
    });

    // Edge case: Empty user story should use default
    it('should use default when user story is empty', () => {
      // ...
    });
    ```

## Test Categories by Code Type

### Pure Functions

```typescript
// Test: inputs → outputs, no side effects
describe('hashArtifact', () => {
  it('should produce deterministic hash', () => {
    const hash1 = hashArtifact(artifact);
    const hash2 = hashArtifact(artifact);
    expect(hash1).toBe(hash2);
  });

  it('should produce different hash for different input', () => {
    const hash1 = hashArtifact(artifact1);
    const hash2 = hashArtifact(artifact2);
    expect(hash1).not.toBe(hash2);
  });
});
```

### Classes/Objects

```typescript
// Test: state management, method interactions
describe('FormatAdapter', () => {
  let adapter: SpecKitAdapter;

  beforeEach(() => {
    adapter = new SpecKitAdapter();
  });

  it('should initialize with correct name', () => {
    expect(adapter.name).toBe('speckit-import');
  });

  it('should detect valid format', () => {
    expect(adapter.canHandle(validSource)).toBe(true);
  });
});
```

### Async Operations

```typescript
// Test: promises, async/await, timing
describe('async operations', () => {
  it('should resolve with correct data', async () => {
    const result = await loadPlan('spec.md');
    expect(result).toMatchObject({ metadata: expect.any(Object) });
  });

  it('should timeout after 5 seconds', async () => {
    await expect(slowOperation()).rejects.toThrow('Operation timed out');
  }, 6000);
});
```

### Parsers

```typescript
// Test: various input formats, malformed data
describe('parseSpecKit', () => {
  it('should parse valid spec.md', () => {
    const result = parseSpecKit(validSpec);
    expect(result.metadata.title).toBe('Expected Title');
  });

  it('should handle missing optional sections', () => {
    const result = parseSpecKit(minimalSpec);
    expect(result).toBeDefined();
  });

  it('should reject malformed input', () => {
    expect(() => parseSpecKit(malformed)).toThrow('Invalid format');
  });
});
```

## Coverage Targets for Anvil

- **Core packages** (`@anvil/core`): 100% coverage required
- **Adapters** (`@anvil/adapters`): 80%+ coverage
- **CLI** (`cli/`): 70%+ coverage
- **Gate** (`@anvil/gate`): 90%+ coverage

## Example Output

```
🧪 Test Coverage Report

📄 File: packages/adapters/src/speckit/parser.ts

Coverage before: 73%
Coverage after: 94%

✅ Added test cases:
  - parseSpecKitV2: multiline metadata (edge case)
  - parseSpecKitV2: missing optional fields (edge case)
  - parseSpecKitV2: invalid format detection (error case)
  - extractUserStory: empty string handling (edge case)
  - extractUserStory: special characters (edge case)

📊 Coverage breakdown:
  Lines: 95% (122/128)
  Branches: 92% (45/49)
  Functions: 100% (12/12)

⚠️ Remaining gaps:
  - Line 87: Unreachable error handler (by design)
  - Lines 104-106: Deprecated V1 fallback (to be removed)

✅ All tests passing: 23 total (18 existing + 5 new)
```

## Anvil Project Specifics

- Test framework: Vitest (configured in vite.config.ts)
- Test location: Co-located with source or in `__tests__/`
- Naming: `*.test.ts` or `*.spec.ts`
- Run: `pnpm test`, `pnpm test:coverage`, `pnpm test:ui`
- Mock utilities: `packages/adapters/src/base/testing.ts`
- Fixtures: Keep in `__tests__/fixtures/` directory
