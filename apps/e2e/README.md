# E2E Test Suites

> **Status:** Placeholder for v1.1+

End-to-end test suites for all Anvil applications.

## Structure

```
e2e/
├── cli-e2e/        # CLI integration tests (migrate from e2e/)
├── api-e2e/        # API endpoint tests
├── ui-e2e/         # UI interaction tests
├── website-e2e/    # Website smoke tests
├── docs-e2e/       # Documentation link/example validation
└── oss-compat-e2e/ # OSS compatibility matrix tests
```

## Current State

The existing E2E tests remain at the root `e2e/` directory until migration.

## Tech Stack

- Playwright for all suites
- Shared fixtures and utilities
