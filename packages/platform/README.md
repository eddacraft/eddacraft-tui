# Platform Packages

> **Status:** Placeholder for v1.1+

Cross-cutting infrastructure concerns extracted from core.

## Structure

```
platform/
├── config/      # Configuration loading and validation
├── storage/     # File system and persistence abstractions
├── telemetry/   # Logging, metrics, tracing
├── auth/        # Authentication and authorisation
├── crypto/      # Hashing, signing, verification
└── http/        # HTTP client abstractions
```

## Migration Plan

These packages will be extracted from current locations:

| New Package | Source                            |
| ----------- | --------------------------------- |
| config      | `core/src/config/`                |
| storage     | `core/src/fs/`, `core/src/cache/` |
| telemetry   | New (consolidate logging)         |
| auth        | New                               |
| crypto      | `core/src/crypto/`                |
| http        | New                               |

## Design Principles

- No domain logic
- Pluggable implementations
- Environment-agnostic interfaces
