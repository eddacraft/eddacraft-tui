# @eddacraft/anvil-\* Core Packages

Layered architecture packages for the Anvil core domain.

## Structure

```
anvil/
├── contracts/   # @eddacraft/anvil-contracts - Schemas, types (zero deps)
├── ports/       # @eddacraft/anvil-ports - Interface definitions
├── core/        # @eddacraft/anvil-core - Pure domain logic (no I/O)
├── runtime/     # @eddacraft/anvil-runtime - Orchestration and I/O
├── policy/      # @eddacraft/anvil-policy - OPA/Rego policy wrappers
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

### @eddacraft/anvil-policy (Layer 2)

OPA/Rego integration for policy evaluation.

```typescript
import {
  OPAExecutor,
  BundleManager,
  PolicyLoader,
} from '@eddacraft/anvil-policy';
```

### @eddacraft/anvil-runtime (Layer 3)

Orchestration and I/O operations.

```typescript
import { GateRunner, FileCache, FileWatcher } from '@eddacraft/anvil-runtime';
```

## Dependency Direction

```
apps → runtime → core → ports → contracts
          ↓
       policy ───────────────→ contracts
```

## Migration Status

| Package   | Status   | Source                        |
| --------- | -------- | ----------------------------- |
| contracts | Scaffold | core/src/schema/, types/      |
| ports     | Scaffold | core/src/gate/check.interface |
| core      | Scaffold | core/src/antipattern/, etc.   |
| runtime   | Scaffold | core/src/gate/, cache/, watch |
| policy    | Scaffold | core/src/gate/policy/         |
| sdk       | Planned  | Not yet created               |

Note: Scaffolds are in place. File migration to be completed.
