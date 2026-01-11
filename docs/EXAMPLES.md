# Anvil Examples

Real-world examples showing Anvil catching issues and workflows for common
scenarios.

## Table of Contents

- [Catching Anti-Patterns](#catching-anti-patterns)
- [Architecture Violations](#architecture-violations)
- [Suppression Examples](#suppression-examples)
- [Development Workflows](#development-workflows)
- [CI/CD Integration](#cicd-integration)
- [Team Workflows](#team-workflows)

## Catching Anti-Patterns

### Example 1: Explicit `any` Type

AI coding assistants often use `any` to "make things work" without proper
typing.

**Problematic Code** (`src/api/handler.ts`):

```typescript
export async function handleRequest(req: any, res: any) {
  const data = req.body as any;
  const result = await processData(data);
  res.json(result);
}

function processData(input: any): any {
  return input.map((item: any) => item.value);
}
```

**Anvil Output**:

```bash
$ anvil check src/api/handler.ts --verbose

Warnings:

⚠ [AP-003] Explicit any type detected
  src/api/handler.ts:1
  Using 'any' defeats type safety
  Why: The 'any' type disables TypeScript's type checking entirely
  Fix: Define a proper type or use 'unknown'

⚠ [AP-003] Explicit any type detected
  src/api/handler.ts:2
  Using 'any' defeats type safety

⚠ [AP-003] Explicit any type detected
  src/api/handler.ts:7
  Using 'any' defeats type safety

Summary:
  Total: 3
  Warnings: 3
  Time: 32ms
```

**Fixed Code**:

```typescript
interface Request {
  body: RequestBody;
}

interface Response {
  json: (data: ProcessedData[]) => void;
}

interface RequestBody {
  items: DataItem[];
}

interface DataItem {
  value: string;
}

interface ProcessedData {
  value: string;
}

export async function handleRequest(req: Request, res: Response) {
  const data = req.body;
  const result = await processData(data.items);
  res.json(result);
}

function processData(input: DataItem[]): ProcessedData[] {
  return input.map((item) => ({ value: item.value }));
}
```

---

### Example 2: Broad ESLint Disable

AI tools sometimes add broad disables to silence multiple warnings at once.

**Problematic Code** (`src/legacy/importer.ts`):

```typescript
/* eslint-disable */
import { legacyProcess } from './legacy';

export function importData(data) {
  console.log('Importing:', data);
  return legacyProcess(data);
}
```

**Anvil Output**:

```bash
$ anvil check src/legacy/importer.ts

Errors:
  ✗ [AP-001] Broad eslint-disable found
    src/legacy/importer.ts:1
    Disabling all ESLint rules in this file
    Fix: Use specific rule disables: /* eslint-disable specific-rule */

Summary:
  Total: 1
  Errors: 1
  Time: 18ms

✗ Blocking warnings found (severity: error)
```

**Fixed Code**:

```typescript
/* eslint-disable @typescript-eslint/no-explicit-any */
// Legacy importer requires any for backward compatibility
import { legacyProcess } from './legacy';

export function importData(data: unknown): ProcessedData {
  // eslint-disable-next-line no-console
  console.log('Importing:', data);
  return legacyProcess(data);
}
```

---

### Example 3: Empty Catch Block

Silent error swallowing is a common AI-generated anti-pattern.

**Problematic Code** (`src/utils/cache.ts`):

```typescript
export async function getCached(key: string): Promise<Data | null> {
  try {
    const cached = await redis.get(key);
    return cached ? JSON.parse(cached) : null;
  } catch {}
  return null;
}
```

**Anvil Output**:

```bash
$ anvil check src/utils/cache.ts --verbose

Warnings:
  ⚠ [AP-006] Empty catch block found
    src/utils/cache.ts:6
    Errors are silently swallowed
    Why: Silent error handling hides bugs and makes debugging difficult
    Fix: Log the error or handle it explicitly

Summary:
  Total: 1
  Warnings: 1
  Time: 22ms
```

**Fixed Code** (Option A - Log error):

```typescript
import { logger } from './logger';

export async function getCached(key: string): Promise<Data | null> {
  try {
    const cached = await redis.get(key);
    return cached ? JSON.parse(cached) : null;
  } catch (error) {
    logger.warn('Cache get failed', { key, error });
    return null;
  }
}
```

**Fixed Code** (Option B - Suppress with explanation):

```typescript
export async function getCached(key: string): Promise<Data | null> {
  try {
    const cached = await redis.get(key);
    return cached ? JSON.parse(cached) : null;
    // @anvil-ignore AP-006: Cache miss is expected; fallback to null is intentional
  } catch {}
  return null;
}
```

---

### Example 4: @ts-ignore Directive

**Problematic Code** (`src/integrations/external.ts`):

```typescript
import { ExternalSDK } from 'external-sdk';

export function initializeSDK(config: Config): SDK {
  // @ts-ignore
  const sdk = new ExternalSDK(config);
  // @ts-ignore
  sdk.configure({ timeout: 5000 });
  return sdk;
}
```

**Anvil Output**:

```bash
$ anvil check src/integrations/external.ts

Warnings:
  ⚠ [AP-004] @ts-ignore directive found
    src/integrations/external.ts:5
    Type error ignored without fixing root cause

  ⚠ [AP-004] @ts-ignore directive found
    src/integrations/external.ts:7
    Type error ignored without fixing root cause

Summary:
  Total: 2
  Warnings: 2
  Time: 19ms
```

**Fixed Code** (with proper types):

```typescript
import { ExternalSDK, SDKConfig, SDKInstance } from 'external-sdk';

export function initializeSDK(config: SDKConfig): SDKInstance {
  const sdk = new ExternalSDK(config);
  sdk.configure({ timeout: 5000 });
  return sdk;
}
```

**Or with justified suppression**:

```typescript
import { ExternalSDK } from 'external-sdk';

export function initializeSDK(config: Config): SDK {
  // @anvil-ignore AP-004: @types/external-sdk missing, PR submitted upstream
  // @ts-ignore - SDK v2.0 types not yet available
  const sdk = new ExternalSDK(config);
  // @anvil-ignore AP-004: Same issue as above
  // @ts-ignore
  sdk.configure({ timeout: 5000 });
  return sdk;
}
```

---

## Architecture Violations

### Example 5: Cross-Layer Dependency

**Scenario**: API handler directly accessing database, bypassing service layer.

**Problematic Code** (`src/api/users.ts`):

```typescript
import { db } from '../database/connection'; // Direct DB access!
import { UserSchema } from '../database/schemas/user';

export async function getUser(req: Request, res: Response) {
  const user = await db.query(UserSchema).findById(req.params.id);
  res.json(user);
}
```

**Anvil Output**:

```bash
$ anvil check src/api/users.ts

Warnings:
  ⚠ [ARCH-001] New cross-boundary dependency
    src/api/users.ts → src/database/connection.ts
    API layer directly accessing database layer
    Why: Bypasses service layer, makes testing difficult
    Fix: Use UserService from src/services/user.service.ts

Summary:
  Total: 1
  Warnings: 1
  Time: 45ms
```

**Fixed Code**:

```typescript
import { UserService } from '../services/user.service';

const userService = new UserService();

export async function getUser(req: Request, res: Response) {
  const user = await userService.findById(req.params.id);
  res.json(user);
}
```

---

## Suppression Examples

### When Suppression is Appropriate

```typescript
// External library with incorrect types
// @anvil-ignore AP-004: lodash types incomplete for this overload
// @ts-ignore
import { merge } from 'lodash';

// Intentional any for dynamic plugin system
// @anvil-ignore AP-003: Plugin interface is intentionally dynamic
type Plugin = { name: string; execute: (data: any) => any };

// Legacy code migration in progress
// @anvil-ignore AP-001: File pending TypeScript migration (JIRA-1234)
/* eslint-disable */

// Performance-critical code with expected errors
// @anvil-ignore AP-006: JSON.parse failure is expected for invalid cache
try {
  return JSON.parse(cached);
} catch {}
```

### When NOT to Suppress

```typescript
// ❌ Don't suppress without reason
// @anvil-ignore AP-003
const data: any = fetchData();

// ❌ Don't suppress to avoid fixing
// @anvil-ignore AP-004: I'll fix this later
// @ts-ignore
const result = brokenFunction();

// ❌ Don't suppress broad disables
// @anvil-ignore AP-001: Too many errors
/* eslint-disable */
```

---

## Development Workflows

### Workflow 1: Fix Issues as You Code

```bash
# Start watch mode
anvil watch --source

# In another terminal, edit files
# Anvil shows warnings in real-time:

[14:32:05] Change detected: src/api/handler.ts
[14:32:05] ⚠ 2 warnings (31ms)
           AP-003: Explicit any type detected (line 5)
           AP-003: Explicit any type detected (line 12)

# Fix the issues, save again:

[14:33:12] Change detected: src/api/handler.ts
[14:33:12] ✓ 0 warnings (28ms)
```

### Workflow 2: Pre-Commit Check

```bash
# Stage your changes
git add src/api/handler.ts

# Check staged files
anvil check --changed --staged

# If warnings found, fix them or suppress with explanation
# Then commit
git commit -m "feat: add user handler"
```

### Workflow 3: PR Review Preparation

```bash
# Check all changes against main branch
anvil check --changed --since main --verbose

# Review warnings, fix or document suppressions
# Then push
git push origin feature/user-handler
```

---

## CI/CD Integration

### GitHub Actions

**`.github/workflows/anvil.yml`**:

```yaml
name: Anvil Check

on:
  pull_request:
    types: [opened, synchronize, reopened]

permissions:
  contents: read
  pull-requests: write
  checks: write

jobs:
  anvil:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0 # Needed for --since comparison

      - uses: ./.github/actions/anvil-check
        with:
          fail-on-warnings: 'false'
```

**PR Comment Output**:

```markdown
## Anvil Check Results

| Status      | Count    |
| ----------- | -------- |
| ⚠️ Warnings | 2        |
| ✅ Passed   | 0 errors |

### Warnings

- **AP-003** `src/api/handler.ts:5` - Explicit any type detected
- **AP-006** `src/utils/cache.ts:12` - Empty catch block found

[View Details](#)
```

### GitLab CI

**`.gitlab-ci.yml`**:

```yaml
anvil:
  stage: test
  image: node:22
  script:
    - npm install -g @anvil/cli
    - anvil check --changed --since $CI_MERGE_REQUEST_TARGET_BRANCH_NAME --json
      > report.json
  artifacts:
    reports:
      codequality: report.json
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

---

## Team Workflows

### Workflow: Gradual Adoption

**Week 1**: Install, run in warning mode

```bash
anvil init --non-interactive
anvil check --changed  # See current state
```

**Week 2**: Enable pre-commit hooks

```bash
anvil hooks install
# Developers see warnings before commit
```

**Week 3**: Add CI integration (non-blocking)

```yaml
- uses: ./.github/actions/anvil-check
  with:
    fail-on-warnings: 'false'
```

**Week 4+**: Review metrics, consider blocking

```yaml
- uses: ./.github/actions/anvil-check
  with:
    fail-on-warnings: 'true' # Now blocking
```

### Workflow: Handling Legacy Code

**Create baseline** (acknowledges existing issues):

```bash
# Run check and save baseline
anvil check src/ --json > .anvil/baseline.json
```

**Configure to warn on NEW issues only**:

```json
{
  "checks": {
    "architecture": {
      "baseline": ".anvil/baseline.json"
    }
  }
}
```

**Track progress**:

```bash
# Compare current state to baseline
anvil status

# Shows:
# Baseline: 45 known issues
# Current:  42 issues (-3 fixed)
# New:      0 issues
```

---

## Real-World Scenario: AI Code Review

**Scenario**: AI generated this authentication handler:

```typescript
/* eslint-disable */
import jwt from 'jsonwebtoken';

export async function authenticate(req: any, res: any) {
  try {
    const token = req.headers.authorization?.split(' ')[1];
    // @ts-ignore
    const decoded = jwt.verify(token, process.env.JWT_SECRET);
    req.user = decoded as any;
    return true;
  } catch {
    return false;
  }
}
```

**Anvil catches all issues**:

```bash
$ anvil check src/auth/authenticate.ts --verbose

Errors:
  ✗ [AP-001] Broad eslint-disable found
    src/auth/authenticate.ts:1
    Fix: Use specific rule disables

Warnings:
  ⚠ [AP-003] Explicit any type detected
    src/auth/authenticate.ts:4
    Fix: Define Request and Response types

  ⚠ [AP-003] Explicit any type detected
    src/auth/authenticate.ts:4
    Fix: Define Request and Response types

  ⚠ [AP-004] @ts-ignore directive found
    src/auth/authenticate.ts:8
    Fix: Add proper type for jwt.verify result

  ⚠ [AP-003] Explicit any type detected
    src/auth/authenticate.ts:9
    Fix: Define User type

  ⚠ [AP-006] Empty catch block found
    src/auth/authenticate.ts:11
    Fix: Log authentication failures

Summary:
  Total: 6
  Errors: 1
  Warnings: 5
  Time: 38ms

✗ Blocking warnings found
```

**Fixed by developer**:

```typescript
import jwt, { JwtPayload } from 'jsonwebtoken';
import { Request, Response } from 'express';
import { logger } from '../utils/logger';

interface AuthenticatedRequest extends Request {
  user?: UserPayload;
}

interface UserPayload extends JwtPayload {
  userId: string;
  email: string;
}

export async function authenticate(
  req: AuthenticatedRequest,
  _res: Response
): Promise<boolean> {
  try {
    const authHeader = req.headers.authorization;
    if (!authHeader?.startsWith('Bearer ')) {
      return false;
    }

    const token = authHeader.split(' ')[1];
    const secret = process.env.JWT_SECRET;

    if (!secret) {
      throw new Error('JWT_SECRET not configured');
    }

    const decoded = jwt.verify(token, secret) as UserPayload;
    req.user = decoded;
    return true;
  } catch (error) {
    logger.debug('Authentication failed', { error });
    return false;
  }
}
```

**Final check**:

```bash
$ anvil check src/auth/authenticate.ts
✓ No warnings found
```

---

## Next Steps

- **[User Guide](./USER_GUIDE.md)** — Complete command reference
- **[Troubleshooting](./TROUBLESHOOTING.md)** — Common issues
- **[Architecture](./ARCHITECTURE.md)** — System design

---

**Version**: 1.0.0 | **Last Updated**: January 2026
