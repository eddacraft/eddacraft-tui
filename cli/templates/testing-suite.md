---
id: testing-suite
name: Testing Suite Setup
description: Set up comprehensive testing infrastructure
category: testing
tags: [testing, vitest, jest, e2e, coverage]
variables:
  - name: framework
    description: Testing framework (vitest, jest)
    default: vitest
    required: false
  - name: coverage_threshold
    description: Minimum coverage percentage
    default: 80
    required: false
  - name: e2e_framework
    description: E2E testing framework (playwright, cypress)
    default: playwright
    required: false
---

# Testing Suite Setup

## Intent

Set up comprehensive testing infrastructure with {{ framework }},
{{ e2e_framework }}, and {{ coverage_threshold }}% coverage threshold.

## Changes

### 1. Configure Test Framework

- **File**: `{{ framework }}.config.ts`
- **Action**: Create
- **Description**: {{ framework }} configuration with coverage

### 2. Configure E2E Tests

- **File**: `{{ e2e_framework }}.config.ts`
- **Action**: Create
- **Description**: {{ e2e_framework }} configuration

### 3. Create Test Utilities

- **File**: `src/test-utils/index.ts`
- **Action**: Create
- **Description**: Common test utilities, mocks, fixtures

### 4. Create Test Setup

- **File**: `src/test-utils/setup.ts`
- **Action**: Create
- **Description**: Global test setup and teardown

### 5. Add Sample Tests

- **File**: `src/__tests__/sample.test.ts`
- **Action**: Create
- **Description**: Example test file demonstrating patterns

### 6. Update Package Scripts

- **File**: `package.json`
- **Action**: Modify
- **Description**: Add test, test:coverage, test:e2e scripts

### 7. Add CI Configuration

- **File**: `.github/workflows/test.yml`
- **Action**: Create
- **Description**: CI pipeline for automated testing

## Configuration

```typescript
// {{ framework }}.config.ts
export default {
  test: {
    environment: 'node',
    coverage: {
      enabled: true,
      thresholds: {
        lines: {{ coverage_threshold }},
        functions: {{ coverage_threshold }},
        branches: {{ coverage_threshold }},
        statements: {{ coverage_threshold }},
      },
    },
  },
};
```

## Test Structure

```
src/
├── __tests__/           # Integration tests
├── test-utils/          # Test utilities
│   ├── index.ts
│   ├── setup.ts
│   ├── mocks/
│   └── fixtures/
└── components/
    └── Button/
        └── Button.test.ts  # Unit tests co-located
e2e/
├── specs/               # E2E test specs
└── fixtures/            # E2E test data
```

## Scripts

```json
{
  "test": "{{ framework }}",
  "test:watch": "{{ framework }} --watch",
  "test:coverage": "{{ framework }} --coverage",
  "test:e2e": "{{ e2e_framework }} test",
  "test:e2e:ui": "{{ e2e_framework }} test --ui"
}
```

## Test Patterns

### Unit Test

```typescript
describe('MyFunction', () => {
  it('should return expected result', () => {
    expect(myFunction(input)).toBe(expected);
  });
});
```

### Integration Test

```typescript
describe('UserService', () => {
  beforeEach(async () => {
    await setupTestDatabase();
  });

  it('should create user', async () => {
    const user = await userService.create(userData);
    expect(user.id).toBeDefined();
  });
});
```

## Coverage Requirements

- Lines: {{ coverage_threshold }}%
- Functions: {{ coverage_threshold }}%
- Branches: {{ coverage_threshold }}%
- Statements: {{ coverage_threshold }}%

## Acceptance Criteria

- [ ] {{ framework }} configured correctly
- [ ] {{ e2e_framework }} configured correctly
- [ ] Coverage thresholds met
- [ ] CI pipeline passing
- [ ] Test utilities working
- [ ] Sample tests passing
