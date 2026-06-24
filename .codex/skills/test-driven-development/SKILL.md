---
name: test-driven-development
description: TDD workflow for authorised work: red, green, refactor, targeted verification, and evidence capture.
---

# Test-Driven Development Skill

## Overview

This skill enables rigorous test-driven development following the red-green-refactor cycle after planning has defined expected behaviour and validation.

## When to Apply

- Adding new features
- Fixing bugs (write failing test first)
- Refactoring code
- Working on critical business logic

Do not use this skill to resolve unclear scope, behaviour, or architecture. Return
to `planning-workflow` or `aps-planning` when the expected outcome is ambiguous.

## The TDD Cycle

### 1. RED - Write a Failing Test

```
Confirm the desired behaviour from the plan
Write the smallest test that fails
Run the targeted test to confirm it fails
Ensure it fails for the right reason
```

### 2. GREEN - Make It Pass

```
Write the minimum correct code to pass
Don't optimize or clean up yet
Run the test to confirm it passes
Celebrate the green!
```

### 3. REFACTOR - Clean Up

```
Improve the code structure
Remove duplication
Improve naming
Run targeted tests to ensure still green
```

## Test Types

### Unit Tests

- Test single functions/methods in isolation
- Fast (milliseconds)
- No external dependencies
- High coverage target (80%+)

### Integration Tests

- Test component interactions
- May use real databases/services
- Slower but more realistic
- Cover critical paths

### End-to-End Tests

- Test complete user flows
- Slowest, most brittle
- Use sparingly for critical paths

## Best Practices

### Writing Good Tests

1. **One assertion per test** (when possible)
2. **Descriptive names**: `should_return_error_when_input_is_empty`
3. **AAA pattern**: Arrange, Act, Assert
4. **Independent tests**: No shared mutable state
5. **Fast tests**: Mock slow dependencies

### Test Structure

```typescript
describe("FeatureName", () => {
  describe("when condition", () => {
    it("should expected behavior", () => {
      // Arrange
      const input = createTestInput();

      // Act
      const result = functionUnderTest(input);

      // Assert
      expect(result).toEqual(expectedOutput);
    });
  });
});
```

### What to Test

- Happy path scenarios
- Edge cases (empty, null, max values)
- Error conditions
- Boundary conditions
- State transitions
- Regression cases from bug reports or review findings

### What NOT to Test

- Third-party library internals
- Framework code
- Trivial getters/setters
- Private implementation details

## Mocking Guidelines

### When to Mock

- External services (APIs, databases)
- Time-dependent code
- Random number generation
- File system operations

### When NOT to Mock

- The code under test
- Simple value objects
- Pure functions

### Mock Patterns

```typescript
// Dependency injection
const mockService = {
  getData: jest.fn().mockResolvedValue(testData),
};
const result = await functionUnderTest(mockService);

// Verify interactions
expect(mockService.getData).toHaveBeenCalledWith(expectedArgs);
```

## Verification Checklist

Before marking TDD complete:

- [ ] All tests pass
- [ ] Tests fail when implementation is broken
- [ ] Required validation evidence is captured
- [ ] Tests are readable and maintainable
- [ ] No test interdependencies
