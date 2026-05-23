# Product Licensing Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development
> (if subagents available) or superpowers:executing-plans to implement this
> plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add offline-capable product licensing to Anvil — API issues signed JWT
licence blobs during login, CLI validates them locally.

**Architecture:** Two-file model — `auth.json` holds the raw token for API
calls, `~/.anvil/license` holds a signed JWT for offline entitlement. The API
signs with ES256 (private key), the CLI verifies with a baked-in public key. A
background refresh fires after a configurable `rcAfter` window.

**Tech Stack:** `jose` (JWT signing/verification, ESM native, zero deps), Zod
(schema validation), Node `crypto` (existing patterns), Vitest (testing).

**Spec:** `docs/archive/specs/2026-03-12-product-licensing-design.md` (archived 2026-05-23, DOCGOV-008)

---

## File Structure

### New Files

| File                                                             | Responsibility                                            |
| ---------------------------------------------------------------- | --------------------------------------------------------- |
| `apps/anvil-api/src/lib/licence.ts`                              | JWT signing — builds claims, signs with ES256 private key |
| `apps/anvil-api/src/lib/__tests__/licence.test.ts`               | Tests for licence signing                                 |
| `apps/anvil-cli/src/services/licence-store.ts`                   | Licence file CRUD — read, write, delete, resolve path     |
| `apps/anvil-cli/src/services/licence-verifier.ts`                | JWT verification — signature check, expiry, rcAfter       |
| `apps/anvil-cli/src/services/licence-refresh.ts`                 | Background refresh — non-blocking API call, deduplication |
| `apps/anvil-cli/src/services/__tests__/licence-store.test.ts`    | Tests for licence store                                   |
| `apps/anvil-cli/src/services/__tests__/licence-verifier.test.ts` | Tests for licence verification                            |
| `apps/anvil-cli/src/services/__tests__/licence-refresh.test.ts`  | Tests for background refresh                              |
| `scripts/generate-licence-keypair.sh`                            | One-time keypair generation script                        |

### Modified Files

| File                                                   | Change                                                                    |
| ------------------------------------------------------ | ------------------------------------------------------------------------- |
| `apps/anvil-api/src/routes/auth.ts`                    | Extend `/verify` to return licence blob; add `/license/refresh`           |
| `apps/anvil-api/src/__tests__/auth.test.ts`            | Update verify tests, add refresh tests                                    |
| `apps/anvil-cli/src/services/auth-client.ts`           | Add `license: z.string()` to VerifyResponseSchema; add `refreshLicence()` |
| `apps/anvil-cli/src/services/auth-store.ts`            | Export `getAuthDir()` for licence-store reuse                             |
| `apps/anvil-cli/src/commands/login.ts`                 | Save licence file after verify                                            |
| `apps/anvil-cli/src/commands/logout.ts`                | Delete licence file alongside auth                                        |
| `apps/anvil-cli/src/commands/whoami.ts`                | Show licence info (tier, org, expiry, next check)                         |
| `apps/anvil-cli/src/index.ts:76-92`                    | Replace `isAuthenticated()` with licence verification                     |
| `apps/anvil-cli/src/services/template-generator.ts:86` | Add `.anvil/license` to gitignore patterns                                |

---

## Chunk 1: Foundation — Keypair Generation + API Licence Signing

### Task 1: Install `jose` in anvil-api

**Files:**

- Modify: `apps/anvil-api/package.json`

- [ ] **Step 1: Install jose**

```bash
cd apps/anvil-api && pnpm add jose
```

- [ ] **Step 2: Verify installation**

```bash
cd apps/anvil-api && node -e "import('jose').then(j => console.log('jose OK:', Object.keys(j).length, 'exports'))"
```

Expected: `jose OK: <number> exports`

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-api/package.json pnpm-lock.yaml
git commit -m "chore(api): add jose for JWT licence signing"
```

---

### Task 2: Generate ES256 keypair + helper script

**Files:**

- Create: `scripts/generate-licence-keypair.sh`

- [ ] **Step 1: Create the keypair generation script**

```bash
#!/usr/bin/env bash
# generate-licence-keypair.sh — Generate an ES256 keypair for licence signing.
#
# Usage:
#   bash scripts/generate-licence-keypair.sh
#
# Output:
#   Prints PEM-encoded private and public keys to stdout.
#   Copy them into your environment variables:
#     LICENSE_SIGNING_KEY — private key (API only, never commit)
#     LICENSE_PUBLIC_KEY  — public key (API + baked into CLI)

set -euo pipefail

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

openssl ecparam -genkey -name prime256v1 -noout -out "$TMPDIR/private.pem" 2>/dev/null
openssl ec -in "$TMPDIR/private.pem" -pubout -out "$TMPDIR/public.pem" 2>/dev/null

echo "=== LICENSE_SIGNING_KEY (private — API env var only) ==="
echo ""
cat "$TMPDIR/private.pem"
echo ""
echo "=== LICENSE_PUBLIC_KEY (public — baked into CLI + API env var) ==="
echo ""
cat "$TMPDIR/public.pem"
echo ""
echo "Copy these into your environment. NEVER commit the private key."
```

- [ ] **Step 2: Make it executable and test**

```bash
chmod +x scripts/generate-licence-keypair.sh
bash scripts/generate-licence-keypair.sh
```

Expected: Two PEM blocks printed (private + public).

- [ ] **Step 3: Commit**

```bash
git add scripts/generate-licence-keypair.sh
git commit -m "chore: add licence keypair generation script"
```

---

### Task 3: API licence signing module — tests first

**Files:**

- Create: `apps/anvil-api/src/lib/__tests__/licence.test.ts`
- Create: `apps/anvil-api/src/lib/licence.ts`

- [ ] **Step 1: Write the failing tests**

Create `apps/anvil-api/src/lib/__tests__/licence.test.ts`:

```typescript
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { generateKeyPair, importSPKI, jwtVerify } from 'jose';
import { signLicence, type LicenceClaims } from '../licence.js';

let originalSigningKey: string | undefined;
let originalPublicKey: string | undefined;
let testPrivateKeyPem: string;
let testPublicKeyPem: string;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  originalPublicKey = process.env['LICENSE_PUBLIC_KEY'];

  // Generate a test keypair
  const { privateKey, publicKey } = await generateKeyPair('ES256');
  const { exportPKCS8 } = await import('jose');
  const { exportSPKI } = await import('jose');
  testPrivateKeyPem = await exportPKCS8(privateKey);
  testPublicKeyPem = await exportSPKI(publicKey);

  process.env['LICENSE_SIGNING_KEY'] = testPrivateKeyPem;
  process.env['LICENSE_PUBLIC_KEY'] = testPublicKeyPem;
});

afterAll(() => {
  if (originalSigningKey === undefined)
    delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
  if (originalPublicKey === undefined) delete process.env['LICENSE_PUBLIC_KEY'];
  else process.env['LICENSE_PUBLIC_KEY'] = originalPublicKey;
});

function makeClaims(overrides: Partial<LicenceClaims> = {}): LicenceClaims {
  return {
    sub: 'user_test123',
    email: 'test@example.com',
    identity: { provider: 'github', id: null },
    org: null,
    tier: 'pro',
    scopes: ['beta'],
    seats: 1,
    ...overrides,
  };
}

describe('signLicence', () => {
  it('returns a signed JWT string', async () => {
    const jwt = await signLicence(makeClaims());
    expect(typeof jwt).toBe('string');
    // JWT has 3 dot-separated parts
    expect(jwt.split('.').length).toBe(3);
  });

  it('includes correct claims in the payload', async () => {
    const claims = makeClaims({ email: 'josh@eddacraft.ai', tier: 'pro' });
    const jwt = await signLicence(claims);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    expect(payload.email).toBe('josh@eddacraft.ai');
    expect(payload.tier).toBe('pro');
    expect(payload.sub).toBe('user_test123');
    expect(payload.org).toBeNull();
    expect(payload.seats).toBe(1);
  });

  it('sets exp to 90 days from now', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims());
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const ninetyDays = 90 * 24 * 60 * 60;
    expect(payload.exp).toBeGreaterThanOrEqual(before + ninetyDays);
    expect(payload.exp).toBeLessThanOrEqual(after + ninetyDays);
  });

  it('sets rcAfter to 7 days from now', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims());
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const sevenDays = 7 * 24 * 60 * 60;
    expect(payload['rcAfter']).toBeGreaterThanOrEqual(before + sevenDays);
    expect(payload['rcAfter']).toBeLessThanOrEqual(after + sevenDays);
  });

  it('sets kid header to identify the key version', async () => {
    const jwt = await signLicence(makeClaims());
    // Decode header without verification
    const headerB64 = jwt.split('.')[0];
    const header = JSON.parse(Buffer.from(headerB64, 'base64url').toString());
    expect(header.kid).toBeDefined();
    expect(header.alg).toBe('ES256');
  });

  it('throws if LICENSE_SIGNING_KEY is not set', async () => {
    const saved = process.env['LICENSE_SIGNING_KEY'];
    delete process.env['LICENSE_SIGNING_KEY'];
    try {
      await expect(signLicence(makeClaims())).rejects.toThrow(
        'LICENSE_SIGNING_KEY'
      );
    } finally {
      process.env['LICENSE_SIGNING_KEY'] = saved;
    }
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd apps/anvil-api && pnpm vitest run src/lib/__tests__/licence.test.ts
```

Expected: FAIL — `licence.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `apps/anvil-api/src/lib/licence.ts`:

```typescript
import { SignJWT, importPKCS8 } from 'jose';

const LICENCE_TTL_DAYS = 90;
const RC_AFTER_DAYS = 7;
const KEY_ID = '2026-03';

export interface LicenceClaims {
  sub: string;
  email: string;
  identity: { provider: string; id: string | null };
  org: string | null;
  tier: string;
  scopes: string[];
  seats: number;
}

/**
 * Sign a licence JWT with ES256.
 *
 * Requires LICENSE_SIGNING_KEY env var (PEM-encoded EC private key).
 */
export async function signLicence(claims: LicenceClaims): Promise<string> {
  const pem = process.env['LICENSE_SIGNING_KEY'];
  if (!pem) {
    throw new Error('LICENSE_SIGNING_KEY environment variable is required');
  }

  const privateKey = await importPKCS8(pem, 'ES256');
  const now = Math.floor(Date.now() / 1000);

  return new SignJWT({
    email: claims.email,
    identity: claims.identity,
    org: claims.org,
    tier: claims.tier,
    scopes: claims.scopes,
    seats: claims.seats,
    rcAfter: now + RC_AFTER_DAYS * 86400,
  })
    .setProtectedHeader({ alg: 'ES256', kid: KEY_ID })
    .setSubject(claims.sub)
    .setIssuedAt(now)
    .setExpirationTime(now + LICENCE_TTL_DAYS * 86400)
    .sign(privateKey);
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd apps/anvil-api && pnpm vitest run src/lib/__tests__/licence.test.ts
```

Expected: All 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/anvil-api/src/lib/licence.ts apps/anvil-api/src/lib/__tests__/licence.test.ts
git commit -m "feat(api): add licence JWT signing module"
```

---

## Chunk 2: API Route Changes — Verify + Refresh

### Task 4: Extend /auth/verify to return licence blob

**Files:**

- Modify: `apps/anvil-api/src/routes/auth.ts:58-64`
- Modify: `apps/anvil-api/src/__tests__/auth.test.ts`

- [ ] **Step 1: Write the failing test**

Add to the existing `apps/anvil-api/src/__tests__/auth.test.ts` — a new test in
the existing `describe('POST /auth/verify')` block:

```typescript
it('returns a licence JWT on successful verification', async () => {
  // Existing mock setup should already cover a valid token
  const res = await app.request('/api/v1/auth/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token: 'anvil_beta_' + 'a'.repeat(43) }),
  });
  const json = await res.json();
  expect(json.valid).toBe(true);
  expect(json.license).toBeDefined();
  expect(typeof json.license).toBe('string');
  expect(json.license.split('.').length).toBe(3); // JWT format
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd apps/anvil-api && pnpm vitest run src/__tests__/auth.test.ts
```

Expected: FAIL — `json.license` is undefined.

- [ ] **Step 3: Update the verify route**

Modify `apps/anvil-api/src/routes/auth.ts`. Add the import and update the
success response:

```typescript
// Add import at top:
import { signLicence } from '../lib/licence.js';

// Replace the success response block (lines 58-64):
debug('token verified successfully');
const licence = await signLicence({
  sub: record.user_id,
  email: record.email,
  identity: { provider: 'github', id: null },
  org: null,
  tier: 'pro',
  scopes: record.scopes,
  seats: 1,
});

return c.json({
  valid: true,
  user: { email: record.email },
  scopes: record.scopes,
  expiresAt: record.expires_at,
  license: licence,
});
```

- [ ] **Step 4: Update auth.test.ts setup for licence signing**

The test environment needs a `LICENSE_SIGNING_KEY` to sign licence blobs. Add a
`beforeAll` block to `auth.test.ts` (or update the existing one):

```typescript
import { generateKeyPair, exportPKCS8, exportSPKI } from 'jose';

let originalSigningKey: string | undefined;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  const { privateKey } = await generateKeyPair('ES256');
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
});

afterAll(() => {
  if (originalSigningKey === undefined)
    delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
});
```

Also update the existing `'returns valid response for active token'` test —
change its `toEqual` assertion to `expect.objectContaining` since the response
now includes a `license` field:

```typescript
expect(body).toEqual(
  expect.objectContaining({
    valid: true,
    user: { email: 'test@example.com' },
    scopes: ['beta'],
    expiresAt,
  })
);
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd apps/anvil-api && pnpm vitest run src/__tests__/auth.test.ts
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/anvil-api/src/routes/auth.ts apps/anvil-api/src/__tests__/auth.test.ts
git commit -m "feat(api): include licence JWT in verify response"
```

---

### Task 5: Add /auth/license/refresh endpoint

**Files:**

- Modify: `apps/anvil-api/src/routes/auth.ts`
- Modify: `apps/anvil-api/src/__tests__/auth.test.ts`

- [ ] **Step 1: Write the failing tests**

Add a new `describe('POST /auth/license/refresh')` block in
`apps/anvil-api/src/__tests__/auth.test.ts`:

```typescript
describe('POST /auth/license/refresh', () => {
  it('returns a fresh licence JWT for a valid token', async () => {
    const res = await app.request('/api/v1/auth/license/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: 'anvil_beta_' + 'a'.repeat(43) }),
    });
    const json = await res.json();
    expect(json.license).toBeDefined();
    expect(typeof json.license).toBe('string');
  });

  it('returns valid:false for revoked token', async () => {
    // Use a token that maps to a revoked record in the mock
    const res = await app.request('/api/v1/auth/license/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: 'anvil_beta_' + 'r'.repeat(43) }),
    });
    const json = await res.json();
    expect(json.valid).toBe(false);
  });

  it('returns valid:false for invalid token format', async () => {
    const res = await app.request('/api/v1/auth/license/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: 'bad_token' }),
    });
    const json = await res.json();
    expect(json.valid).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd apps/anvil-api && pnpm vitest run src/__tests__/auth.test.ts
```

Expected: FAIL — 404 on `/auth/license/refresh`.

- [ ] **Step 3: Add the refresh endpoint**

Add to `apps/anvil-api/src/routes/auth.ts`, after the `/verify` handler:

```typescript
/**
 * POST /auth/license/refresh
 *
 * Revalidates a token and issues a fresh licence JWT.
 * Called by CLI background refresh when rcAfter has passed.
 */
auth.post('/license/refresh', zValidator('json', verifySchema), async (c) => {
  debug('POST /auth/license/refresh');
  const { token } = c.req.valid('json');

  if (!isValidTokenFormat(token)) {
    return c.json({ valid: false });
  }

  const sql = getClient();
  const hash = hashToken(token);
  const record = await findTokenByHash(sql, hash);

  if (!record || record.revoked_at || record.user_status !== 'active') {
    return c.json({
      valid: false,
      reason: record?.revoked_at ? 'revoked' : 'invalid',
    });
  }

  if (new Date(record.expires_at).getTime() < Date.now()) {
    return c.json({ valid: false, reason: 'expired' });
  }

  const licence = await signLicence({
    sub: record.user_id,
    email: record.email,
    identity: { provider: 'github', id: null },
    org: null,
    tier: 'pro',
    scopes: record.scopes,
    seats: 1,
  });

  return c.json({ license: licence });
});
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd apps/anvil-api && pnpm vitest run src/__tests__/auth.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/anvil-api/src/routes/auth.ts apps/anvil-api/src/__tests__/auth.test.ts
git commit -m "feat(api): add licence refresh endpoint"
```

---

## Chunk 3: CLI Licence Store

### Task 6: Install `jose` in anvil-cli

**Files:**

- Modify: `apps/anvil-cli/package.json`

- [ ] **Step 1: Install jose**

```bash
cd apps/anvil-cli && pnpm add jose
```

- [ ] **Step 2: Commit**

```bash
git add apps/anvil-cli/package.json pnpm-lock.yaml
git commit -m "chore(cli): add jose for JWT licence verification"
```

---

### Task 7: Export getAuthDir from auth-store

**Files:**

- Modify: `apps/anvil-cli/src/services/auth-store.ts:25-27`

- [ ] **Step 1: Export the function**

In `apps/anvil-cli/src/services/auth-store.ts`, change line 25 from:

```typescript
function getAuthDir(): string {
```

to:

```typescript
export function getAuthDir(): string {
```

- [ ] **Step 2: Verify existing tests still pass**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/auth-store.test.ts
```

Expected: All tests PASS (no behavioural change).

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-cli/src/services/auth-store.ts
git commit -m "refactor(cli): export getAuthDir for licence store reuse"
```

---

### Task 8: Licence store — tests first

**Files:**

- Create: `apps/anvil-cli/src/services/__tests__/licence-store.test.ts`
- Create: `apps/anvil-cli/src/services/licence-store.ts`

- [ ] **Step 1: Write the failing tests**

Create `apps/anvil-cli/src/services/__tests__/licence-store.test.ts`:

```typescript
import { describe, it, expect, afterEach, beforeAll, afterAll } from 'vitest';
import {
  mkdtempSync,
  writeFileSync,
  existsSync,
  readFileSync,
  rmSync,
  statSync,
} from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setAuthDir } from '../auth-store.js';
import {
  loadLicence,
  saveLicence,
  clearLicence,
  resolveLicencePath,
} from '../licence-store.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

let tempDir: string;

beforeAll(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-licence-test-'));
  setAuthDir(tempDir);
});

afterAll(async () => {
  setAuthDir(null);
  if (tempDir && existsSync(tempDir)) {
    await safeCleanup(tempDir);
  }
});

afterEach(() => {
  const licPath = join(tempDir, 'license');
  if (existsSync(licPath)) rmSync(licPath);
  delete process.env['ANVIL_LICENSE'];
});

describe('licence-store', () => {
  describe('saveLicence', () => {
    it('writes the JWT string to the license file', () => {
      saveLicence('eyJhbGciOiJFUzI1NiJ9.test.sig');
      const content = readFileSync(join(tempDir, 'license'), 'utf-8');
      expect(content).toBe('eyJhbGciOiJFUzI1NiJ9.test.sig');
    });

    it.skipIf(process.platform === 'win32')(
      'sets restrictive permissions (0o600)',
      () => {
        saveLicence('test-jwt');
        const stats = statSync(join(tempDir, 'license'));
        expect(stats.mode & 0o777).toBe(0o600);
      }
    );
  });

  describe('loadLicence', () => {
    it('returns null when no license file exists', () => {
      expect(loadLicence()).toBeNull();
    });

    it('returns the JWT string when file exists', () => {
      saveLicence('my.jwt.token');
      expect(loadLicence()).toBe('my.jwt.token');
    });

    it('trims whitespace from the file content', () => {
      writeFileSync(join(tempDir, 'license'), '  my.jwt.token  \n');
      expect(loadLicence()).toBe('my.jwt.token');
    });
  });

  describe('clearLicence', () => {
    it('deletes the license file', () => {
      saveLicence('test');
      clearLicence();
      expect(existsSync(join(tempDir, 'license'))).toBe(false);
    });

    it('does not throw if no file exists', () => {
      expect(() => clearLicence()).not.toThrow();
    });
  });

  describe('resolveLicencePath', () => {
    it('returns ANVIL_LICENSE env var path when set and file exists', () => {
      const envPath = join(tempDir, 'env-license');
      writeFileSync(envPath, 'env-jwt');
      process.env['ANVIL_LICENSE'] = envPath;
      expect(resolveLicencePath()).toBe(envPath);
    });

    it('falls back to user-level when env var file does not exist', () => {
      process.env['ANVIL_LICENSE'] = '/nonexistent/path';
      saveLicence('user-jwt');
      expect(resolveLicencePath()).toBe(join(tempDir, 'license'));
    });

    it('returns user-level path when no env var set', () => {
      saveLicence('user-jwt');
      expect(resolveLicencePath()).toBe(join(tempDir, 'license'));
    });

    it('returns null when no license file exists anywhere', () => {
      expect(resolveLicencePath()).toBeNull();
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-store.test.ts
```

Expected: FAIL — `licence-store.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `apps/anvil-cli/src/services/licence-store.ts`:

```typescript
import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  unlinkSync,
  existsSync,
  chmodSync,
} from 'node:fs';
import { join } from 'node:path';
import { getAuthDir } from './auth-store.js';
import { debug } from '../utils/output.js';

const LICENCE_FILENAME = 'license';

function getUserLicencePath(): string {
  return join(getAuthDir(), LICENCE_FILENAME);
}

/**
 * Resolve the licence file path.
 * Priority: ANVIL_LICENSE env var → project-level → user-level.
 * Returns null if no licence file exists at any location.
 */
export function resolveLicencePath(projectRoot?: string): string | null {
  // 1. Environment variable
  const envPath = process.env['ANVIL_LICENSE'];
  if (envPath && existsSync(envPath)) return envPath;

  // 2. Project-level
  if (projectRoot) {
    const projectPath = join(projectRoot, '.anvil', LICENCE_FILENAME);
    if (existsSync(projectPath)) return projectPath;
  }

  // 3. User-level
  const userPath = getUserLicencePath();
  if (existsSync(userPath)) return userPath;

  return null;
}

/**
 * Load the licence JWT string from the resolved path.
 */
export function loadLicence(projectRoot?: string): string | null {
  const path = resolveLicencePath(projectRoot);
  if (!path) return null;

  try {
    return readFileSync(path, 'utf-8').trim();
  } catch {
    debug('loadLicence: failed to read licence file');
    return null;
  }
}

/**
 * Save a licence JWT string to the user-level licence file.
 */
export function saveLicence(jwt: string): void {
  const dir = getAuthDir();
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true, mode: 0o700 });
  }

  const path = getUserLicencePath();
  writeFileSync(path, jwt, { mode: 0o600 });
  chmodSync(path, 0o600);
}

/**
 * Delete the user-level licence file.
 */
export function clearLicence(): void {
  const path = getUserLicencePath();
  if (existsSync(path)) {
    unlinkSync(path);
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-store.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/anvil-cli/src/services/licence-store.ts apps/anvil-cli/src/services/__tests__/licence-store.test.ts
git commit -m "feat(cli): add licence file store with resolution order"
```

---

## Chunk 4: CLI Licence Verification

### Task 9: Licence verifier — tests first

**Files:**

- Create: `apps/anvil-cli/src/services/__tests__/licence-verifier.test.ts`
- Create: `apps/anvil-cli/src/services/licence-verifier.ts`

- [ ] **Step 1: Write the failing tests**

Create `apps/anvil-cli/src/services/__tests__/licence-verifier.test.ts`:

```typescript
import { describe, it, expect, beforeAll } from 'vitest';
import {
  generateKeyPair,
  exportPKCS8,
  exportSPKI,
  SignJWT,
  importPKCS8,
} from 'jose';
import {
  verifyLicence,
  setPublicKeys,
  type LicenceResult,
} from '../licence-verifier.js';

let testPrivateKeyPem: string;
let testPublicKeyPem: string;

async function signTestJwt(
  claims: Record<string, unknown>,
  options: { kid?: string; exp?: number } = {}
): Promise<string> {
  const privateKey = await importPKCS8(testPrivateKeyPem, 'ES256');
  const now = Math.floor(Date.now() / 1000);

  const builder = new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256', kid: options.kid ?? '2026-03' })
    .setSubject((claims.sub as string) ?? 'user_test')
    .setIssuedAt(now)
    .setExpirationTime(options.exp ?? now + 86400);

  return builder.sign(privateKey);
}

beforeAll(async () => {
  const { privateKey, publicKey } = await generateKeyPair('ES256');
  testPrivateKeyPem = await exportPKCS8(privateKey);
  testPublicKeyPem = await exportSPKI(publicKey);

  setPublicKeys({ '2026-03': testPublicKeyPem });
});

describe('verifyLicence', () => {
  it('returns valid result for a correctly signed JWT', async () => {
    const jwt = await signTestJwt({
      email: 'test@example.com',
      tier: 'pro',
      org: null,
      rcAfter: Math.floor(Date.now() / 1000) + 86400,
    });

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(true);
    expect(result.claims?.email).toBe('test@example.com');
    expect(result.claims?.tier).toBe('pro');
    expect(result.needsRefresh).toBe(false);
  });

  it('returns needsRefresh when rcAfter has passed', async () => {
    const jwt = await signTestJwt({
      email: 'test@example.com',
      tier: 'pro',
      rcAfter: Math.floor(Date.now() / 1000) - 100, // in the past
    });

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(true);
    expect(result.needsRefresh).toBe(true);
  });

  it('returns invalid for an expired JWT', async () => {
    const jwt = await signTestJwt(
      { email: 'test@example.com', tier: 'pro', rcAfter: 0 },
      { exp: Math.floor(Date.now() / 1000) - 100 }
    );

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('expired');
  });

  it('returns invalid for a tampered JWT', async () => {
    const jwt = await signTestJwt({
      email: 'test@example.com',
      tier: 'pro',
      rcAfter: 0,
    });
    const tampered = jwt.slice(0, -5) + 'XXXXX';

    const result = await verifyLicence(tampered);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('invalid_signature');
  });

  it('returns invalid for a JWT signed with an unknown kid', async () => {
    const jwt = await signTestJwt(
      { email: 'test@example.com', tier: 'pro', rcAfter: 0 },
      { kid: 'unknown-key' }
    );

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('unknown_key');
  });

  it('returns invalid for garbage input', async () => {
    const result = await verifyLicence('not-a-jwt');
    expect(result.valid).toBe(false);
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-verifier.test.ts
```

Expected: FAIL — `licence-verifier.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `apps/anvil-cli/src/services/licence-verifier.ts`:

```typescript
import {
  jwtVerify,
  importSPKI,
  decodeProtectedHeader,
  errors,
  type JWTPayload,
} from 'jose';
import { debug } from '../utils/output.js';

export interface LicenceClaims {
  sub: string;
  email: string;
  identity?: { provider: string; id: string | null };
  org: string | null;
  tier: string;
  scopes?: string[];
  seats?: number;
  rcAfter: number;
  exp: number;
}

export type LicenceResult =
  | { valid: true; claims: LicenceClaims; needsRefresh: boolean }
  | { valid: false; reason: string };

/**
 * Public keys keyed by kid. In production these are baked into the CLI binary.
 * Use setPublicKeys() in tests to inject test keys.
 */
let publicKeysPem: Record<string, string> = {};

/** Inject public keys (for testing or build-time baking). */
export function setPublicKeys(keys: Record<string, string>): void {
  publicKeysPem = keys;
}

/**
 * Verify a licence JWT locally.
 *
 * 1. Decode the header to get the kid
 * 2. Look up the public key by kid
 * 3. Verify the signature + expiry
 * 4. Check rcAfter to determine if a refresh is needed
 */
export async function verifyLicence(jwt: string): Promise<LicenceResult> {
  try {
    // Decode header to get kid
    let header: { kid?: string };
    try {
      header = decodeProtectedHeader(jwt);
    } catch {
      return { valid: false, reason: 'malformed' };
    }

    const kid = header.kid;
    if (!kid || !publicKeysPem[kid]) {
      debug(`verifyLicence: unknown kid "${kid}"`);
      return { valid: false, reason: 'unknown_key' };
    }

    const publicKey = await importSPKI(publicKeysPem[kid], 'ES256');
    const { payload } = await jwtVerify(jwt, publicKey);

    const now = Math.floor(Date.now() / 1000);
    const rcAfter = (payload as JWTPayload & { rcAfter?: number }).rcAfter ?? 0;
    const needsRefresh = now > rcAfter;

    return {
      valid: true,
      claims: {
        sub: payload.sub ?? '',
        email: (payload as Record<string, unknown>).email as string,
        identity: (payload as Record<string, unknown>).identity as
          | { provider: string; id: string | null }
          | undefined,
        org: ((payload as Record<string, unknown>).org as string) ?? null,
        tier: (payload as Record<string, unknown>).tier as string,
        scopes: (payload as Record<string, unknown>).scopes as
          | string[]
          | undefined,
        seats: (payload as Record<string, unknown>).seats as number | undefined,
        rcAfter,
        exp: payload.exp ?? 0,
      },
      needsRefresh,
    };
  } catch (err) {
    if (err instanceof errors.JWTExpired) {
      return { valid: false, reason: 'expired' };
    }
    debug(`verifyLicence: verification failed: ${err}`);
    return { valid: false, reason: 'invalid_signature' };
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-verifier.test.ts
```

Expected: All 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/anvil-cli/src/services/licence-verifier.ts apps/anvil-cli/src/services/__tests__/licence-verifier.test.ts
git commit -m "feat(cli): add offline licence JWT verification"
```

---

## Chunk 5: CLI Background Refresh

### Task 10: Background refresh + deduplication — tests first

**Files:**

- Create: `apps/anvil-cli/src/services/__tests__/licence-refresh.test.ts`
- Create: `apps/anvil-cli/src/services/licence-refresh.ts`

- [ ] **Step 1: Write the failing tests**

Create `apps/anvil-cli/src/services/__tests__/licence-refresh.test.ts`:

```typescript
import {
  describe,
  it,
  expect,
  vi,
  beforeEach,
  afterEach,
  beforeAll,
  afterAll,
} from 'vitest';
import {
  mkdtempSync,
  existsSync,
  readFileSync,
  writeFileSync,
  rmSync,
} from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { setAuthDir } from '../auth-store.js';
import { saveLicence, loadLicence } from '../licence-store.js';
import {
  scheduleRefresh,
  getLastRefreshAttempt,
  REFRESH_COOLDOWN_MS,
  _setRefreshStatePath,
} from '../licence-refresh.js';
import { safeCleanup } from '../../../../../tools/test-utils/safe-cleanup.js';

let tempDir: string;

beforeAll(() => {
  tempDir = mkdtempSync(join(tmpdir(), 'anvil-refresh-test-'));
  setAuthDir(tempDir);
  _setRefreshStatePath(join(tempDir, 'refresh-state'));
});

afterAll(async () => {
  setAuthDir(null);
  _setRefreshStatePath(null);
  if (tempDir && existsSync(tempDir)) {
    await safeCleanup(tempDir);
  }
});

afterEach(() => {
  for (const f of ['license', 'refresh-state', 'auth.json']) {
    const p = join(tempDir, f);
    if (existsSync(p)) rmSync(p);
  }
  vi.restoreAllMocks();
});

describe('licence-refresh', () => {
  describe('getLastRefreshAttempt', () => {
    it('returns 0 when no refresh-state file exists', () => {
      expect(getLastRefreshAttempt()).toBe(0);
    });

    it('returns the stored timestamp', () => {
      writeFileSync(join(tempDir, 'refresh-state'), '1700000000000');
      expect(getLastRefreshAttempt()).toBe(1700000000000);
    });
  });

  describe('scheduleRefresh', () => {
    it('skips refresh when last attempt was within cooldown', async () => {
      // Write a recent timestamp
      writeFileSync(join(tempDir, 'refresh-state'), String(Date.now()));

      const mockFetch = vi.fn();
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('skipped_cooldown');
      expect(mockFetch).not.toHaveBeenCalled();
    });

    it('calls the API and updates licence on success', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ license: 'new.jwt.token' }),
      });

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('refreshed');
      expect(loadLicence()).toBe('new.jwt.token');
    });

    it('deletes licence when API returns valid:false', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ valid: false, reason: 'revoked' }),
      });

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('revoked');
      expect(loadLicence()).toBeNull();
    });

    it('returns error on network failure without touching licence', async () => {
      const mockFetch = vi.fn().mockRejectedValue(new Error('Network error'));

      saveLicence('old.jwt.token');
      const result = await scheduleRefresh('anvil_beta_test', mockFetch);
      expect(result).toBe('error');
      expect(loadLicence()).toBe('old.jwt.token');
    });

    it('persists the refresh attempt timestamp', async () => {
      const mockFetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({ license: 'new.jwt.token' }),
      });

      const before = Date.now();
      await scheduleRefresh('anvil_beta_test', mockFetch);
      const after = Date.now();

      const ts = getLastRefreshAttempt();
      expect(ts).toBeGreaterThanOrEqual(before);
      expect(ts).toBeLessThanOrEqual(after);
    });
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-refresh.test.ts
```

Expected: FAIL — `licence-refresh.js` does not exist.

- [ ] **Step 3: Write the implementation**

Create `apps/anvil-cli/src/services/licence-refresh.ts`:

```typescript
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { getAuthDir } from './auth-store.js';
import { saveLicence, clearLicence } from './licence-store.js';
import { getApiUrl } from './api-client.js';
import { debug } from '../utils/output.js';

export const REFRESH_COOLDOWN_MS = 60_000; // 60 seconds

/** Override for testing. */
let _refreshStatePathOverride: string | null = null;

/** Set a custom refresh state path (for testing). */
export function _setRefreshStatePath(path: string | null): void {
  _refreshStatePathOverride = path;
}

function getRefreshStatePath(): string {
  return _refreshStatePathOverride ?? join(getAuthDir(), 'refresh-state');
}

/** Read the last refresh attempt timestamp (ms since epoch). */
export function getLastRefreshAttempt(): number {
  const path = getRefreshStatePath();
  try {
    if (!existsSync(path)) return 0;
    const raw = readFileSync(path, 'utf-8').trim();
    return parseInt(raw, 10) || 0;
  } catch {
    return 0;
  }
}

function saveRefreshAttempt(): void {
  writeFileSync(getRefreshStatePath(), String(Date.now()));
}

export type RefreshResult =
  | 'refreshed'
  | 'revoked'
  | 'skipped_cooldown'
  | 'error';

type FetchFn = typeof globalThis.fetch;

/**
 * Attempt a licence refresh.
 *
 * @param token - The raw beta token from auth.json
 * @param fetchFn - Injectable fetch for testing (defaults to global fetch)
 * @returns The result of the refresh attempt
 */
export async function scheduleRefresh(
  token: string,
  fetchFn: FetchFn = globalThis.fetch
): Promise<RefreshResult> {
  // Deduplication: skip if refreshed recently
  const lastAttempt = getLastRefreshAttempt();
  if (Date.now() - lastAttempt < REFRESH_COOLDOWN_MS) {
    debug('licence refresh: skipped (cooldown)');
    return 'skipped_cooldown';
  }

  saveRefreshAttempt();

  try {
    const apiUrl = getApiUrl();
    const response = await fetchFn(`${apiUrl}/api/v1/auth/license/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
      signal: AbortSignal.timeout(10_000),
    });

    const data = (await response.json()) as {
      license?: string;
      valid?: boolean;
    };

    if (data.license) {
      saveLicence(data.license);
      debug('licence refresh: success');
      return 'refreshed';
    }

    if (data.valid === false) {
      clearLicence();
      debug('licence refresh: revoked');
      return 'revoked';
    }

    debug('licence refresh: unexpected response');
    return 'error';
  } catch (err) {
    debug(`licence refresh: failed: ${err}`);
    return 'error';
  }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-refresh.test.ts
```

Expected: All 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/anvil-cli/src/services/licence-refresh.ts apps/anvil-cli/src/services/__tests__/licence-refresh.test.ts
git commit -m "feat(cli): add background licence refresh with deduplication"
```

---

## Chunk 6: CLI Command Integration

### Task 11: Update auth-client to capture licence from verify response

**Files:**

- Modify: `apps/anvil-cli/src/services/auth-client.ts`

- [ ] **Step 1: Update the VerifyResponseSchema**

In `apps/anvil-cli/src/services/auth-client.ts`, change the schema to:

```typescript
const VerifyResponseSchema = z.object({
  valid: z.boolean(),
  user: z.object({ email: z.string() }).optional(),
  scopes: z.array(z.string()).optional(),
  expiresAt: z.string().optional(),
  license: z.string().optional(),
});
```

Note: `.optional()` here rather than required — this allows the CLI to work
against older API versions that don't yet return `license`. The login command
will check for its presence explicitly. **Intentional spec deviation:** the spec
says `z.string()` (required), but `.optional()` is safer during the rollout
window when the API and CLI may be deployed at different times.

- [ ] **Step 2: Verify existing tests still pass**

```bash
cd apps/anvil-cli && pnpm vitest run
```

Expected: No regressions.

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-cli/src/services/auth-client.ts
git commit -m "feat(cli): add licence field to verify response schema"
```

---

### Task 12: Update login command to save licence

**Files:**

- Modify: `apps/anvil-cli/src/commands/login.ts`

- [ ] **Step 1: Update login.ts**

Add the import and update the success handler. In
`apps/anvil-cli/src/commands/login.ts`:

Add import at top:

```typescript
import { saveLicence } from '../services/licence-store.js';
```

After the `saveAuth(...)` call (around line 68), add:

```typescript
if (result.license) {
  saveLicence(result.license);
}
```

Update the success message (replace lines 71-73):

```typescript
success(`Authenticated as ${chalk.bold(result.user.email)}`);
info(`Scopes: ${result.scopes.join(', ')}`);
if (result.license) {
  info('Licence saved locally for offline verification');
}
info(`Expires: ${new Date(result.expiresAt).toLocaleString()}`);
```

- [ ] **Step 2: Verify the CLI still works**

```bash
cd apps/anvil-cli && pnpm vitest run
```

Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-cli/src/commands/login.ts
git commit -m "feat(cli): save licence blob on login"
```

---

### Task 13: Update logout to clear licence

**Files:**

- Modify: `apps/anvil-cli/src/commands/logout.ts`

- [ ] **Step 1: Update logout.ts**

Replace the full file content:

```typescript
import { Command } from 'commander';
import { clearAuth, loadAuth } from '../services/auth-store.js';
import { clearLicence } from '../services/licence-store.js';
import { success, info } from '../utils/output.js';

export function createLogoutCommand(): Command {
  const command = new Command('logout');

  command.description('Clear stored credentials').action(() => {
    const existing = loadAuth();
    clearAuth();
    clearLicence();

    if (existing) {
      success('Logged out. Local credentials removed.');
    } else {
      info('No active session');
    }
  });

  return command;
}
```

- [ ] **Step 2: Verify tests pass**

```bash
cd apps/anvil-cli && pnpm vitest run
```

Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-cli/src/commands/logout.ts
git commit -m "feat(cli): clear licence file on logout"
```

---

### Task 14: Update whoami to show licence info

**Files:**

- Modify: `apps/anvil-cli/src/commands/whoami.ts`

- [ ] **Step 1: Update whoami.ts**

Replace the full file content:

```typescript
import { Command } from 'commander';
import chalk from 'chalk';
import { loadAuth } from '../services/auth-store.js';
import { loadLicence, resolveLicencePath } from '../services/licence-store.js';
import { verifyLicence } from '../services/licence-verifier.js';
import { blank, error, print } from '../utils/output.js';
import { CliError } from '../utils/cli-error.js';

export function createWhoamiCommand(): Command {
  const command = new Command('whoami');

  command
    .description('Display current authentication and licence info')
    .action(async () => {
      const auth = loadAuth();

      if (!auth) {
        error('Not authenticated. Run `anvil login` to authenticate.');
        throw new CliError('Not authenticated');
      }

      print(chalk.bold('\nSession Info\n'));
      print(`  Email:    ${chalk.cyan(auth.user.email)}`);
      print(`  Scopes:   ${auth.scopes.join(', ')}`);
      print(`  Expires:  ${new Date(auth.expiresAt).toLocaleString()}`);
      print(`  Verified: ${new Date(auth.verifiedAt).toLocaleString()}`);

      // Licence info
      const jwt = loadLicence();
      if (jwt) {
        const result = await verifyLicence(jwt);
        if (result.valid) {
          blank();
          print(chalk.bold('Licence\n'));
          print(`  Tier:       ${chalk.cyan(result.claims.tier)}`);
          print(`  Org:        ${result.claims.org ?? chalk.dim('none')}`);
          if (result.claims.identity?.id) {
            print(
              `  Identity:   ${result.claims.identity.provider}:${result.claims.identity.id}`
            );
          }
          print(
            `  Expires:    ${new Date(result.claims.exp * 1000).toLocaleString()}`
          );
          const rcDate = new Date(result.claims.rcAfter * 1000);
          const daysUntilCheck = Math.max(
            0,
            Math.ceil((rcDate.getTime() - Date.now()) / 86400000)
          );
          print(
            `  Next check: ${rcDate.toLocaleDateString()}${daysUntilCheck > 0 ? ` (in ${daysUntilCheck} days)` : chalk.yellow(' (pending)')}`
          );

          const licPath = resolveLicencePath();
          if (licPath) print(`  Licence:    ${chalk.dim(licPath)}`);
        } else {
          blank();
          print(chalk.yellow(`  Licence: invalid (${result.reason})`));
        }
      } else {
        blank();
        print(chalk.dim('  No licence file found'));
      }

      blank();
    });

  return command;
}
```

- [ ] **Step 2: Verify tests pass**

```bash
cd apps/anvil-cli && pnpm vitest run
```

Expected: All tests PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/anvil-cli/src/commands/whoami.ts
git commit -m "feat(cli): show licence details in whoami"
```

---

### Task 15: Update pre-action hook to use licence verification

**Files:**

- Modify: `apps/anvil-cli/src/index.ts:75-92`

- [ ] **Step 1: Update the pre-action hook**

In `apps/anvil-cli/src/index.ts`, replace the existing pre-action hook (lines
75-92). Add new imports at top:

```typescript
import { loadLicence } from './services/licence-store.js';
import { verifyLicence } from './services/licence-verifier.js';
import { scheduleRefresh } from './services/licence-refresh.js';
import { loadAuth } from './services/auth-store.js';
```

Note: `isAuthenticated` is already imported from `auth-store.js` — keep it as a
fallback. Replace the preAction hook body:

```typescript
// Auth gate: check licence before every command (except exempt ones)
program.hook('preAction', async (_thisCommand, actionCommand) => {
  let cmd: Command = actionCommand;
  while (cmd.parent && cmd.parent.parent) {
    cmd = cmd.parent;
  }
  const commandName = cmd.name();

  if (AUTH_EXEMPT_COMMANDS.has(commandName)) return;

  const jwt = loadLicence();
  if (!jwt) {
    // Backwards compat: auth.json exists but no licence
    if (isAuthenticated()) {
      console.error(
        '\x1b[33m!\x1b[0m Your session needs to be refreshed. Run \x1b[1manvil login\x1b[0m to continue.'
      );
    } else {
      console.error(
        '\x1b[31m✗\x1b[0m Authentication required. Run \x1b[1manvil login\x1b[0m to authenticate.\n' +
          '   New here? Try \x1b[1manvil tutorial\x1b[0m first (no login required).'
      );
    }
    throw new CliError('Authentication required');
  }

  const result = await verifyLicence(jwt);

  if (!result.valid) {
    const msg =
      result.reason === 'expired'
        ? 'Your licence needs to be renewed. Run \x1b[1manvil login\x1b[0m to continue.'
        : 'Your licence could not be verified. Run \x1b[1manvil login\x1b[0m or contact support@eddacraft.ai if this is unexpected.';
    console.error(`\x1b[31m✗\x1b[0m ${msg}`);
    throw new CliError('Licence verification failed');
  }

  // Background refresh if needed (non-blocking)
  if (result.needsRefresh) {
    const auth = loadAuth();
    if (auth) {
      scheduleRefresh(auth.token).catch(() => {
        // Swallow — refresh is best-effort
      });
    }
  }
});
```

- [ ] **Step 2: Remove unused isAuthenticated import if no longer needed**

Check if `isAuthenticated` is still used elsewhere in `index.ts`. It's used in
the backwards compat check above, so keep the import.

- [ ] **Step 3: Verify the CLI still builds and tests pass**

```bash
cd apps/anvil-cli && pnpm vitest run
```

Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/anvil-cli/src/index.ts
git commit -m "feat(cli): replace auth check with licence verification in pre-action hook"
```

---

## Chunk 7: Gitignore + Public Key Baking

### Task 16: Add .anvil/license to gitignore template

**Files:**

- Modify: `apps/anvil-cli/src/services/template-generator.ts:86`

- [ ] **Step 1: Update the patterns array**

In `apps/anvil-cli/src/services/template-generator.ts`, find the
`updateGitignore()` method (line 83). Change the patterns array from:

```typescript
const patterns = [
  '',
  '# Anvil',
  '.anvil/cache/',
  '.anvil/evidence/',
  '.anvil/*.log',
];
```

to:

```typescript
const patterns = [
  '',
  '# Anvil',
  '.anvil/cache/',
  '.anvil/evidence/',
  '.anvil/*.log',
  '.anvil/license',
];
```

- [ ] **Step 2: Commit**

```bash
git add apps/anvil-cli/src/services/template-generator.ts
git commit -m "chore(cli): add .anvil/license to gitignore template"
```

---

### Task 17: Bake the public key into the CLI

**Files:**

- Create: `apps/anvil-cli/src/services/licence-keys.ts`

- [ ] **Step 1: Create the public key module**

Create `apps/anvil-cli/src/services/licence-keys.ts`:

```typescript
/**
 * Public keys for licence JWT verification, keyed by kid.
 *
 * These are baked into the CLI at build time. Only the public key is included —
 * it can verify signatures but not create them.
 *
 * To rotate keys:
 * 1. Generate a new keypair: bash scripts/generate-licence-keypair.sh
 * 2. Add the new public key here with a new kid
 * 3. Ship a CLI release containing both keys
 * 4. Update the API to sign with the new private key
 * 5. After all old licences expire (90 days), remove the old key from here
 */
export const LICENCE_PUBLIC_KEYS: Record<string, string> = {
  // Populated from LICENSE_PUBLIC_KEY env var after keypair generation.
  // Replace this placeholder with the real PEM-encoded public key.
  //
  // Example:
  // '2026-03': `-----BEGIN PUBLIC KEY-----
  // MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE...
  // -----END PUBLIC KEY-----`,
};
```

- [ ] **Step 2: Wire it into licence-verifier.ts**

In `apps/anvil-cli/src/services/licence-verifier.ts`, add at the top:

```typescript
import { LICENCE_PUBLIC_KEYS } from './licence-keys.js';
```

And at the bottom of the file, add an initialisation call:

```typescript
// Load baked-in keys on import (tests can override via setPublicKeys)
if (
  Object.keys(publicKeysPem).length === 0 &&
  Object.keys(LICENCE_PUBLIC_KEYS).length > 0
) {
  setPublicKeys(LICENCE_PUBLIC_KEYS);
}
```

- [ ] **Step 3: Verify tests still pass** (tests call `setPublicKeys` before the
      import-time init matters)

```bash
cd apps/anvil-cli && pnpm vitest run src/services/__tests__/licence-verifier.test.ts
```

Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/anvil-cli/src/services/licence-keys.ts apps/anvil-cli/src/services/licence-verifier.ts
git commit -m "feat(cli): bake public key module for licence verification"
```

---

### Task 18: Generate real keypair and wire it up

This is a manual step — not automated in tests.

- [ ] **Step 1: Generate the keypair**

```bash
bash scripts/generate-licence-keypair.sh
```

- [ ] **Step 2: Set the private key in the API environment**

Copy the private key PEM into the `LICENSE_SIGNING_KEY` environment variable in
your Vercel project settings (or `.env.local` for local dev).

- [ ] **Step 3: Paste the public key into licence-keys.ts**

Replace the placeholder in `apps/anvil-cli/src/services/licence-keys.ts` with
the real public key PEM.

- [ ] **Step 4: Verify locally**

Run `anvil login` against a local or staging API that has the private key set.
Then run `anvil whoami` to confirm the licence is verified.

- [ ] **Step 5: Commit** (the public key only — NEVER commit the private key)

```bash
git add apps/anvil-cli/src/services/licence-keys.ts
git commit -m "chore(cli): add production licence public key"
```

---

## Summary

| Task | What                       | Files                                                     |
| ---- | -------------------------- | --------------------------------------------------------- |
| 1    | Install jose (API)         | `apps/anvil-api/package.json`                             |
| 2    | Keypair generation script  | `scripts/generate-licence-keypair.sh`                     |
| 3    | API licence signing module | `apps/anvil-api/src/lib/licence.ts` + tests               |
| 4    | Extend /auth/verify        | `apps/anvil-api/src/routes/auth.ts` + tests               |
| 5    | Add /auth/license/refresh  | `apps/anvil-api/src/routes/auth.ts` + tests               |
| 6    | Install jose (CLI)         | `apps/anvil-cli/package.json`                             |
| 7    | Export getAuthDir          | `apps/anvil-cli/src/services/auth-store.ts`               |
| 8    | Licence store              | `apps/anvil-cli/src/services/licence-store.ts` + tests    |
| 9    | Licence verifier           | `apps/anvil-cli/src/services/licence-verifier.ts` + tests |
| 10   | Background refresh         | `apps/anvil-cli/src/services/licence-refresh.ts` + tests  |
| 11   | Update auth-client schema  | `apps/anvil-cli/src/services/auth-client.ts`              |
| 12   | Update login               | `apps/anvil-cli/src/commands/login.ts`                    |
| 13   | Update logout              | `apps/anvil-cli/src/commands/logout.ts`                   |
| 14   | Update whoami              | `apps/anvil-cli/src/commands/whoami.ts`                   |
| 15   | Update pre-action hook     | `apps/anvil-cli/src/index.ts`                             |
| 16   | Gitignore template         | `apps/anvil-cli/src/services/template-generator.ts`       |
| 17   | Public key module          | `apps/anvil-cli/src/services/licence-keys.ts`             |
| 18   | Wire real keypair          | Manual — env vars + public key                            |
