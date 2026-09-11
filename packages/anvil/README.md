# @eddacraft/anvil-\* Core Packages

Layered architecture packages for the Anvil core domain.

## Structure

```
anvil/
├── contracts/   # @eddacraft/anvil-contracts - Schemas, types (zero deps)
├── ports/       # @eddacraft/anvil-ports - Interface definitions
├── core/        # @eddacraft/anvil-core - Pure domain logic (no I/O)
├── runtime/     # @eddacraft/anvil-runtime - Feature-flag resolution via the supported subpath
└── sdk/         # @eddacraft/anvil-sdk - Client SDK (planned, not yet created)
```

## Packages

### @eddacraft/anvil-contracts (Layer 0)

Zod schemas, types, and events with zero dependencies.

```typescript
import {
  APSPlanSchema,
  WarningSchema,
  type APSPlan,
} from '@eddacraft/anvil-contracts';
```

### @eddacraft/anvil-ports (Layer 1)

Interface definitions depending only on contracts.

```typescript
import {
  ICheck,
  ICacheProvider,
  IStorageProvider,
} from '@eddacraft/anvil-ports';
```

### @eddacraft/anvil-core (Layer 2)

Pure domain logic with no I/O operations.

```typescript
import {
  scanForAntipatterns,
  detectDrift,
  analyzeArchitecture,
} from '@eddacraft/anvil-core';
```

### @eddacraft/anvil-runtime (Layer 3)

Feature-flag resolution for API/docs. The TypeScript cache surface and `./cache`
export were retired (CIB-418). FileWatcher and `@eddacraft/anvil-policy` OPA
wrappers were retired earlier (CIB-370); policy evaluation is
`crates/anvil-policy`.

```typescript
import { resolveFlag } from '@eddacraft/anvil-runtime/feature-flags';
```

## Dependency Direction

```
apps → runtime → core → ports → contracts
```
