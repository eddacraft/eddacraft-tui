# E2E Test Suites

End-to-end test suites for all Anvil applications.

## Structure

```
e2e/
├── cli-e2e/        # CLI integration tests
├── api-e2e/        # API endpoint tests (future)
├── ui-e2e/         # UI interaction tests (future)
├── website-e2e/    # Website smoke tests (future)
├── docs-e2e/       # Documentation link/example validation (future)
└── oss-compat-e2e/ # OSS compatibility matrix tests (future)
```

## Running Tests

```bash
# Run all E2E tests
pnpm test:e2e

# Run with UI mode
pnpm test:e2e:ui

# Run specific suite
pnpm test:e2e --project=cli-e2e
```

## Tech Stack

- Playwright for all suites
- Shared fixtures and utilities
- Test isolation per application

## Migration Status

| Suite   | Status  | Description           |
| ------- | ------- | --------------------- |
| cli-e2e | Pending | CLI integration tests |
| api-e2e | Future  | API endpoint tests    |
| ui-e2e  | Future  | UI interaction tests  |
