# @anvil/\* Core Packages

Layered architecture packages for the Anvil core domain.

## Structure

```
anvil/
├── contracts/   # @anvil/contracts - Schemas, types (zero deps)
├── ports/       # @anvil/ports - Interface definitions
├── core/        # @anvil/core - Pure domain logic (no I/O)
├── runtime/     # @anvil/runtime - Orchestration and I/O
├── policy/      # @anvil/policy - OPA/Rego policy wrappers
└── sdk/         # @anvil/sdk - Client SDK (future)
```

## Packages

### @anvil/contracts (Layer 0)

Zod schemas, types, and events with zero dependencies.

```typescript
import { APSPlanSchema, WarningSchema, type APSPlan } from '@anvil/contracts';
```

### @anvil/ports (Layer 1)

Interface definitions depending only on contracts.

```typescript
import { ICheck, ICacheProvider, IStorageProvider } from '@anvil/ports';
```

### @anvil/core (Layer 2)

Pure domain logic with no I/O operations.

```typescript
import {
  scanForAntipatterns,
  detectDrift,
  analyzeArchitecture,
} from '@anvil/core';
```

### @anvil/policy (Layer 2)

OPA/Rego integration for policy evaluation.

```typescript
import { OPAExecutor, BundleManager, PolicyLoader } from '@anvil/policy';
```

### @anvil/runtime (Layer 3)

Orchestration and I/O operations.

```typescript
import { GateRunner, FileCache, FileWatcher } from '@anvil/runtime';
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
| sdk       | Pending  | New package                   |

Note: Scaffolds are in place. File migration to be completed.
