# Platform Packages

Cross-cutting infrastructure concerns for the Anvil monorepo.

## Structure

```
platform/
├── config/      # @anvil/platform-config - Configuration loading
├── storage/     # @anvil/platform-storage - File system abstractions
├── crypto/      # @anvil/platform-crypto - Hashing, signing
├── telemetry/   # (future) Logging, metrics, tracing
├── auth/        # (future) Authentication and authorisation
└── http/        # (future) HTTP client abstractions
```

## Packages

### @anvil/platform-config

Configuration loading and validation utilities.

```typescript
import { createConfigLoader } from '@anvil/platform-config';

const config = createConfigLoader({ baseDir: '/path/to/project' });
const value = config.get<string>('key');
```

### @anvil/platform-storage

File system and persistence abstractions.

```typescript
import { createFileStorage } from '@anvil/platform-storage';

const storage = createFileStorage('/path/to/data');
await storage.write('file.txt', 'content');
const content = await storage.read('file.txt');
```

### @anvil/platform-crypto

Cryptographic utilities for hashing and verification.

```typescript
import { generateHash, verifyHash, generatePlanId } from '@anvil/platform-crypto';

const hash = generateHash({ key: 'value' });
const isValid = verifyHash({ key: 'value' }, hash);
const planId = generatePlanId(); // 'aps-a1b2c3d4'
```

## Migration Status

| Package   | Status   | Source               |
| --------- | -------- | -------------------- |
| config    | Complete | New                  |
| storage   | Complete | New                  |
| crypto    | Complete | core/src/crypto/     |
| telemetry | Pending  | New                  |
| auth      | Pending  | New                  |
| http      | Pending  | New                  |

## Design Principles

- No domain logic
- Pluggable implementations
- Environment-agnostic interfaces
