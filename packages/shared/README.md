# Shared Packages

> **Status:** Placeholder for v1.1+

Shared utilities used across all packages.

## Structure

```
shared/
├── util/        # General utilities (strings, arrays, etc.)
├── testing/     # Test helpers, fixtures, mocks
└── brand/       # Branded types and type utilities
```

## Guidelines

- No dependencies on other @eddacraft/anvil-\* packages
- Pure functions preferred
- Minimal external dependencies
- Well-documented and tested
