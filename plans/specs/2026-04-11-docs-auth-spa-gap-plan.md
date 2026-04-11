# Docs Auth SPA Gap — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the broken single-build Docusaurus auth gate with a Next.js shell that fronts two separated Docusaurus builds (public + private Anvil), so that code-split chunks for gated content actually cannot be fetched without a valid JWT.

**Architecture:** New `apps/docs-shell/` Next.js 16 App Router project owns `docs.eddacraft.ai`. It hosts the landing page, `/auth/*` routes, and routing middleware matching `/anvil/:path*`. `apps/anvil-docs-private/` is a Docusaurus build containing only the Anvil plugin, configured with `baseUrl: '/anvil/'` so every asset (HTML, JS chunks, images, search index) lives under `/anvil/*` and is caught by the middleware matcher. `apps/docs-public/` is the former `apps/docs-site/` with Anvil + beta plugins removed; it serves Kindling, APS, edda-stack, and blog. The shell rewrites `/anvil/*`, `/kindling/*`, `/aps/*`, `/edda-stack/*`, and `/blog/*` to the respective upstream Vercel projects.

**Tech Stack:** Next.js 16, React 19, Node.js 24 LTS, `jose` for JWT, `vitest` for tests, Docusaurus 3.10, Pulumi TypeScript for infra.

**Spec reference:** `plans/specs/2026-04-11-docs-auth-spa-gap-design.md`

---

## File Structure

**Create:**
- `apps/docs-shell/package.json` — Next.js 16 project manifest
- `apps/docs-shell/tsconfig.json`
- `apps/docs-shell/next.config.ts` — rewrites to upstream projects
- `apps/docs-shell/middleware.ts` — JWT gate for `/anvil/:path*`
- `apps/docs-shell/vercel.json`
- `apps/docs-shell/project.json` — nx project config
- `apps/docs-shell/vitest.config.ts`
- `apps/docs-shell/app/layout.tsx` — root layout
- `apps/docs-shell/app/page.tsx` — landing page (ported from `apps/docs-site/src/pages/index.tsx`)
- `apps/docs-shell/app/globals.css` — landing styles
- `apps/docs-shell/app/robots.txt/route.ts`
- `apps/docs-shell/app/llms.txt/route.ts`
- `apps/docs-shell/app/auth/login/route.ts`
- `apps/docs-shell/app/auth/callback/route.ts`
- `apps/docs-shell/app/auth/logout/route.ts`
- `apps/docs-shell/app/auth/pending/page.tsx`
- `apps/docs-shell/app/auth/error/page.tsx`
- `apps/docs-shell/lib/jwt.ts` — public key cache + verify helper
- `apps/docs-shell/lib/jwt.test.ts`
- `apps/docs-shell/lib/state.ts` — AES-256-GCM OAuth state encryption
- `apps/docs-shell/lib/state.test.ts`
- `apps/docs-shell/lib/next-url.ts` — validate `next` query param
- `apps/docs-shell/lib/next-url.test.ts`
- `apps/docs-shell/lib/bauth.ts` — BAUTH callback fetch wrapper
- `apps/docs-shell/lib/bauth.test.ts`
- `apps/docs-shell/lib/cookie.ts` — cookie header parsing (Edge-safe)
- `apps/docs-shell/lib/cookie.test.ts`
- `apps/docs-shell/middleware.test.ts`
- `apps/anvil-docs-private/package.json` — Docusaurus project manifest
- `apps/anvil-docs-private/docusaurus.config.ts` — `baseUrl: '/anvil/'`, Anvil plugin only
- `apps/anvil-docs-private/project.json`
- `apps/anvil-docs-private/tsconfig.json`
- `apps/anvil-docs-private/vercel.json`
- `apps/anvil-docs-private/sidebars/anvil.ts` — copied from docs-site
- `apps/anvil-docs-private/src/css/custom.css` — copied minimal subset
- `apps/anvil-docs-private/static/img/favicon.svg` — copied
- `plans/modules/docs-auth-spa-gap.aps.md` — DOCSAUTH2 APS module tracker

**Modify:**
- `apps/docs-site/docusaurus.config.ts` — remove anvil, beta plugins; remove middleware reference
- `apps/docs-site/package.json` — rename to `@eddacraft/docs-public`
- `apps/docs-site/src/pages/index.tsx` — **delete** (migrated to shell)
- `apps/docs-site/middleware.ts` — **delete**
- `apps/docs-site/api/auth/login.ts` — **delete**
- `apps/docs-site/api/auth/callback.ts` — **delete**
- `apps/docs-site/api/auth/logout.ts` — **delete**
- `apps/docs-site/TOGGLING-DOCS.md` — **delete** (stale)
- `apps/docs-site/vercel.json` — remove function config for middleware/auth
- `infra/src/vercel.ts` — add two new Vercel projects, update docs-site project, reassign domain
- `CLAUDE.md` — add DOCSAUTH2 to active modules, mark DOCSAUTH Complete
- `pnpm-workspace.yaml` — no change (wildcard covers new dirs)

**Rename:**
- Consider renaming `apps/docs-site/` → `apps/docs-public/` as final cleanup task (deferred to cutover).

---

## Task 0 — Kill-switch: verify Docusaurus `baseUrl` prefixes all assets

The entire architecture depends on Docusaurus emitting every asset under `/anvil/*` when `baseUrl: '/anvil/'` is set. If any asset escapes the prefix (sitemap, OG images, search index), the whole plan fails and we fall back to a subdomain split. Verify on a throwaway clone **before** writing any code.

**Files:**
- Throwaway: `/tmp/docusaurus-baseurl-check/` (not committed)

- [ ] **Step 1: Clone docs-site to /tmp for isolation**

```bash
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd /tmp
rm -rf docusaurus-baseurl-check
cp -r "${REPO_ROOT}/apps/docs-site" docusaurus-baseurl-check
cd docusaurus-baseurl-check
rm -rf node_modules .docusaurus build
```

- [ ] **Step 2: Strip to Anvil-only and set baseUrl**

Edit `/tmp/docusaurus-baseurl-check/docusaurus.config.ts`:
- Change `baseUrl: '/',` to `baseUrl: '/anvil/',`
- Remove APS, Kindling, edda-stack, beta plugin entries
- Remove the `docs: false` preset comment if distracting
- Leave the Anvil plugin entry intact

- [ ] **Step 3: Remove the custom homepage**

```bash
rm /tmp/docusaurus-baseurl-check/src/pages/index.tsx
rm /tmp/docusaurus-baseurl-check/src/pages/index.module.css 2>/dev/null || true
rm /tmp/docusaurus-baseurl-check/src/pages/markdown-page.md 2>/dev/null || true
```

- [ ] **Step 4: Install and build**

```bash
cd /tmp/docusaurus-baseurl-check
pnpm install --ignore-workspace
pnpm run build
```

Expected: build succeeds with output in `build/`.

- [ ] **Step 5: Inspect build output for asset path leaks**

```bash
# All JS chunks should be under build/anvil/assets/js/
find /tmp/docusaurus-baseurl-check/build -name "*.js" | head -20
# Expected: all paths start with build/anvil/assets/

# No assets should escape the /anvil/ prefix
find /tmp/docusaurus-baseurl-check/build -type f -not -path "*/anvil/*" | grep -v "^.*\.\(html\|txt\|xml\)$" || echo "CLEAN"
# Expected: CLEAN (only index.html, robots.txt, sitemap.xml allowed at root)

# Check the root HTML references assets under /anvil/
grep -o 'src="[^"]*"' /tmp/docusaurus-baseurl-check/build/anvil/index.html | head -5
# Expected: all src values start with /anvil/assets/
```

- [ ] **Step 6: Serve and smoke-test**

```bash
cd /tmp/docusaurus-baseurl-check
npx http-server build -p 9090 &
sleep 2
curl -s -o /dev/null -w "%{http_code} %{url_effective}\n" http://localhost:9090/anvil/
curl -s http://localhost:9090/anvil/ | grep -o '/anvil/assets/js/[^"]*\.js' | head -5
# Expected: 200 on /anvil/, all chunk URLs prefixed with /anvil/assets/js/
kill %1
```

- [ ] **Step 7: Record the verdict**

Append to `plans/specs/2026-04-11-docs-auth-spa-gap-design.md` under a new "Kill-switch verification" section:

```markdown
## Kill-switch verification (2026-04-11)

- `baseUrl: '/anvil/'` correctly prefixes: HTML routes, JS chunks, CSS, static images, search index, sitemap.
- No assets escape the `/anvil/` prefix in the build output.
- **Verdict: GREEN — proceeding with Next.js shell architecture.**
```

If any asset leaks, **stop** and reopen brainstorming to switch to the subdomain-split fallback (Approach 1 bare variant).

- [ ] **Step 8: Commit the verdict**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
git add plans/specs/2026-04-11-docs-auth-spa-gap-design.md
git commit -m "docs(docsauth): record baseUrl kill-switch verification"
```

---

## Task 1 — Scaffold `apps/docs-shell` project manifest

**Files:**
- Create: `apps/docs-shell/package.json`
- Create: `apps/docs-shell/tsconfig.json`
- Create: `apps/docs-shell/project.json`
- Create: `apps/docs-shell/vitest.config.ts`

- [ ] **Step 1: Write package.json**

```json
{
  "name": "@eddacraft/docs-shell",
  "version": "0.3.0-beta",
  "private": true,
  "scripts": {
    "dev": "next dev -p 3100",
    "build": "next build",
    "start": "next start",
    "lint": "eslint .",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "jose": "^6.2.2",
    "next": "16.2.3",
    "react": "19.2.4",
    "react-dom": "19.2.4"
  },
  "devDependencies": {
    "@types/node": "^25",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "typescript": "~6.0.2",
    "vitest": "^4.1.3"
  }
}
```

- [ ] **Step 2: Write tsconfig.json**

```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "jsx": "preserve",
    "module": "esnext",
    "moduleResolution": "bundler",
    "target": "es2022",
    "lib": ["dom", "dom.iterable", "esnext"],
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "incremental": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules", ".next"]
}
```

- [ ] **Step 3: Write project.json (nx)**

Match the workspace convention used by `apps/website/project.json` — shell
commands with `cwd`, not `nx:run-script` executors.

```json
{
  "name": "docs-shell",
  "$schema": "../../node_modules/nx/schemas/project-schema.json",
  "sourceRoot": "apps/docs-shell",
  "projectType": "application",
  "targets": {
    "dev": { "command": "next dev -p 3100", "options": { "cwd": "apps/docs-shell" } },
    "build": { "command": "next build", "options": { "cwd": "apps/docs-shell" } },
    "start": { "command": "next start", "options": { "cwd": "apps/docs-shell" } },
    "lint": { "command": "eslint .", "options": { "cwd": "apps/docs-shell" } },
    "typecheck": { "command": "tsc --noEmit", "options": { "cwd": "apps/docs-shell" } },
    "test": { "command": "vitest run", "options": { "cwd": "apps/docs-shell" } }
  },
  "tags": ["app", "docs"]
}
```

- [ ] **Step 4: Write vitest.config.ts**

```ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: ['**/*.test.ts', '**/*.test.tsx'],
    exclude: ['node_modules', '.next', 'dist'],
  },
});
```

- [ ] **Step 5: Install dependencies and verify scaffolding**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
pnpm install
pnpm nx typecheck docs-shell
```

Expected: typecheck succeeds (no source files yet, just config).

- [ ] **Step 6: Commit**

```bash
git add apps/docs-shell/package.json apps/docs-shell/tsconfig.json apps/docs-shell/project.json apps/docs-shell/vitest.config.ts pnpm-lock.yaml
git commit -m "feat(docs-shell): scaffold Next.js app manifest"
```

---

## Task 2 — `lib/cookie.ts` (Edge-safe cookie parsing)

Edge runtime lacks `document.cookie`, so we parse raw cookie headers directly. Used by middleware and callback route.

**Files:**
- Create: `apps/docs-shell/lib/cookie.ts`
- Test: `apps/docs-shell/lib/cookie.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// apps/docs-shell/lib/cookie.test.ts
import { describe, it, expect } from 'vitest';
import { getCookie } from './cookie';

describe('getCookie', () => {
  it('returns undefined for null header', () => {
    expect(getCookie(null, 'session')).toBeUndefined();
  });

  it('returns undefined when cookie absent', () => {
    expect(getCookie('other=1; foo=bar', 'session')).toBeUndefined();
  });

  it('extracts a single cookie value', () => {
    expect(getCookie('session=abc123', 'session')).toBe('abc123');
  });

  it('extracts a cookie from a multi-cookie header', () => {
    expect(getCookie('a=1; session=abc123; b=2', 'session')).toBe('abc123');
  });

  it('handles leading whitespace', () => {
    expect(getCookie('a=1;   session=abc123', 'session')).toBe('abc123');
  });

  it('url-decodes the value', () => {
    expect(getCookie('session=%2Fanvil%2Foverview', 'session')).toBe('/anvil/overview');
  });

  it('returns undefined when decoding fails', () => {
    expect(getCookie('session=%ZZ', 'session')).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd apps/docs-shell && pnpm test lib/cookie.test.ts
```

Expected: FAIL with "Cannot find module './cookie'".

- [ ] **Step 3: Write minimal implementation**

```ts
// apps/docs-shell/lib/cookie.ts
export function getCookie(cookieHeader: string | null, name: string): string | undefined {
  if (!cookieHeader) return undefined;
  const match = cookieHeader.match(new RegExp(`(?:^|;\\s*)${name}=([^;]*)`));
  if (!match) return undefined;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return undefined;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test lib/cookie.test.ts
```

Expected: 7 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/docs-shell/lib/cookie.ts apps/docs-shell/lib/cookie.test.ts
git commit -m "feat(docs-shell): add edge-safe cookie header parser"
```

---

## Task 3 — `lib/next-url.ts` (next param validation)

Prevents open redirect by ensuring the `next` query parameter resolves to a `/anvil/` path and contains no protocol or `//`.

**Files:**
- Create: `apps/docs-shell/lib/next-url.ts`
- Test: `apps/docs-shell/lib/next-url.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// apps/docs-shell/lib/next-url.test.ts
import { describe, it, expect } from 'vitest';
import { validateNext } from './next-url';

describe('validateNext', () => {
  it('returns default for null', () => {
    expect(validateNext(null)).toBe('/anvil/overview');
  });

  it('returns default for empty string', () => {
    expect(validateNext('')).toBe('/anvil/overview');
  });

  it('accepts /anvil/overview', () => {
    expect(validateNext('/anvil/overview')).toBe('/anvil/overview');
  });

  it('accepts deep /anvil paths', () => {
    expect(validateNext('/anvil/quickstart/setup')).toBe('/anvil/quickstart/setup');
  });

  it('rejects /kindling path (non-anvil)', () => {
    expect(validateNext('/kindling/overview')).toBe('/anvil/overview');
  });

  it('rejects protocol-relative URLs', () => {
    expect(validateNext('//evil.com/anvil/foo')).toBe('/anvil/overview');
  });

  it('rejects absolute URLs with protocol', () => {
    expect(validateNext('https://evil.com/anvil/foo')).toBe('/anvil/overview');
  });

  it('strips dot-segments and revalidates', () => {
    expect(validateNext('/anvil/../kindling')).toBe('/anvil/overview');
  });

  it('accepts /anvil with trailing segment', () => {
    expect(validateNext('/anvil/')).toBe('/anvil/');
  });

  it('rejects /anvil prefix near-miss like /anvilicious', () => {
    expect(validateNext('/anvilicious')).toBe('/anvil/overview');
  });

  it('accepts bare /anvil', () => {
    expect(validateNext('/anvil')).toBe('/anvil');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test lib/next-url.test.ts
```

Expected: FAIL with "Cannot find module './next-url'".

- [ ] **Step 3: Write minimal implementation**

```ts
// apps/docs-shell/lib/next-url.ts
const DEFAULT_NEXT = '/anvil/overview';

export function validateNext(next: string | null | undefined): string {
  if (!next) return DEFAULT_NEXT;
  if (!next.startsWith('/')) return DEFAULT_NEXT;
  if (next.startsWith('//')) return DEFAULT_NEXT;
  try {
    const resolved = new URL(next, 'https://placeholder.invalid').pathname;
    if (resolved !== '/anvil' && !resolved.startsWith('/anvil/')) return DEFAULT_NEXT;
    return resolved;
  } catch {
    return DEFAULT_NEXT;
  }
}
```

> **Why the `startsWith('/')` guard matters:** `new URL('https://evil.com/anvil/foo', base)` ignores the placeholder base because the input is already absolute, so its `.pathname` becomes `/anvil/foo` — which would sail through the prefix check. Rejecting anything that doesn't start with `/` before parsing blocks `https://…`, `http://…`, `javascript:…`, and bare identifiers, while leaving every valid path case unaffected.

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test lib/next-url.test.ts
```

Expected: 11 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/docs-shell/lib/next-url.ts apps/docs-shell/lib/next-url.test.ts
git commit -m "feat(docs-shell): add next param validator"
```

---

## Task 4 — `lib/state.ts` (OAuth state encrypt/decrypt)

AES-256-GCM roundtrip for the `state` param so GitHub's CSRF state can carry a validated `next` URL and nonce. Uses Web Crypto so it runs on the Edge runtime.

**Files:**
- Create: `apps/docs-shell/lib/state.ts`
- Test: `apps/docs-shell/lib/state.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// apps/docs-shell/lib/state.test.ts
import { describe, it, expect, beforeAll } from 'vitest';
import { encryptState, decryptState } from './state';

const SECRET = 'test-secret-at-least-32-bytes-long-for-aes';

describe('state encryption', () => {
  it('roundtrips a payload', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const encrypted = await encryptState(payload, SECRET);
    const decrypted = await decryptState(encrypted, SECRET);
    expect(decrypted).toEqual(payload);
  });

  it('produces different ciphertext for the same input (random IV)', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const a = await encryptState(payload, SECRET);
    const b = await encryptState(payload, SECRET);
    expect(a).not.toBe(b);
  });

  it('returns null when decrypting with the wrong secret', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const encrypted = await encryptState(payload, SECRET);
    const decrypted = await decryptState(encrypted, 'different-secret-also-long-enough');
    expect(decrypted).toBeNull();
  });

  it('returns null for garbled input', async () => {
    expect(await decryptState('not-valid-base64url', SECRET)).toBeNull();
  });

  it('returns null for truncated input', async () => {
    expect(await decryptState('AA', SECRET)).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test lib/state.test.ts
```

Expected: FAIL with "Cannot find module './state'".

- [ ] **Step 3: Write implementation**

```ts
// apps/docs-shell/lib/state.ts
// AES-256-GCM state encryption using Web Crypto (Edge-compatible).
// Layout: iv(12 bytes) || ciphertext || tag(16 bytes, appended by subtle.encrypt)

export interface StatePayload {
  next: string;
  nonce: string;
}

async function getKey(secret: string): Promise<CryptoKey> {
  const hash = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(secret));
  return crypto.subtle.importKey('raw', hash, { name: 'AES-GCM' }, false, ['encrypt', 'decrypt']);
}

function base64urlEncode(bytes: Uint8Array): string {
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

function base64urlDecode(str: string): Uint8Array<ArrayBuffer> {
  // Return type must be narrowed to Uint8Array<ArrayBuffer> (not the default
  // Uint8Array<ArrayBufferLike>). crypto.subtle.decrypt expects BufferSource,
  // which TS 5.7+ resolves to ArrayBufferView<ArrayBuffer> — the ArrayBufferLike
  // form (which includes SharedArrayBuffer) is rejected as incompatible.
  // Narrowing here propagates through subarray() at the call site.
  const padded = str.replaceAll('-', '+').replaceAll('_', '/') + '='.repeat((4 - (str.length % 4)) % 4);
  const bin = atob(padded);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

export async function encryptState(payload: StatePayload, secret: string): Promise<string> {
  const key = await getKey(secret);
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const plaintext = new TextEncoder().encode(JSON.stringify(payload));
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, plaintext)
  );
  const combined = new Uint8Array(iv.length + ciphertext.length);
  combined.set(iv, 0);
  combined.set(ciphertext, iv.length);
  return base64urlEncode(combined);
}

export async function decryptState(encrypted: string, secret: string): Promise<StatePayload | null> {
  try {
    const combined = base64urlDecode(encrypted);
    if (combined.length < 12 + 16) return null;
    const iv = combined.subarray(0, 12);
    const ciphertext = combined.subarray(12);
    const key = await getKey(secret);
    const plaintext = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext);
    const parsed = JSON.parse(new TextDecoder().decode(plaintext));
    if (typeof parsed?.next !== 'string' || typeof parsed?.nonce !== 'string') return null;
    return { next: parsed.next, nonce: parsed.nonce };
  } catch {
    return null;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test lib/state.test.ts
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/docs-shell/lib/state.ts apps/docs-shell/lib/state.test.ts
git commit -m "feat(docs-shell): add AES-GCM state encryption helpers"
```

---

## Task 5 — `lib/jwt.ts` (ES256 JWT verification)

Caches the ES256 public key across invocations to avoid re-importing on every request. Wraps `jose.jwtVerify` with a consistent return shape.

**Files:**
- Create: `apps/docs-shell/lib/jwt.ts`
- Test: `apps/docs-shell/lib/jwt.test.ts`

- [ ] **Step 1: Generate a test key pair for the test file**

Run this once to generate test keys, then paste into the test:

```bash
node -e "
const { generateKeyPairSync } = require('node:crypto');
const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' });
console.log('PRIVATE:');
console.log(privateKey.export({ type: 'pkcs8', format: 'pem' }));
console.log('PUBLIC:');
console.log(publicKey.export({ type: 'spki', format: 'pem' }));
"
```

Copy the output for use in the test.

- [ ] **Step 2: Write the failing test**

```ts
// apps/docs-shell/lib/jwt.test.ts
import { describe, it, expect, beforeAll } from 'vitest';
import { SignJWT, importPKCS8 } from 'jose';
import { verifyLicense, resetKeyCache } from './jwt';

// Paste the test keys from Step 1 below
const TEST_PUBLIC_KEY_PEM = `-----BEGIN PUBLIC KEY-----
...
-----END PUBLIC KEY-----`;

const TEST_PRIVATE_KEY_PEM = `-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----`;

async function signToken(claims: Record<string, unknown>, expSeconds: number = 3600): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  return new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSeconds)
    .sign(privateKey);
}

describe('verifyLicense', () => {
  beforeAll(() => {
    process.env.LICENSE_PUBLIC_KEY = TEST_PUBLIC_KEY_PEM;
    resetKeyCache();
  });

  it('verifies a valid token', async () => {
    const token = await signToken({ sub: 'user@example.com' });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(true);
  });

  it('rejects an expired token', async () => {
    const token = await signToken({ sub: 'user@example.com' }, -60);
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a garbled token', async () => {
    const result = await verifyLicense('not.a.jwt');
    expect(result.valid).toBe(false);
  });

  it('rejects a token signed with a different key', async () => {
    const token = 'eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiJmb28ifQ.signature';
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });
});
```

- [ ] **Step 3: Run test to verify it fails**

```bash
pnpm test lib/jwt.test.ts
```

Expected: FAIL with "Cannot find module './jwt'".

- [ ] **Step 4: Write implementation**

```ts
// apps/docs-shell/lib/jwt.ts
// NOTE: jose v6 no longer exports `KeyLike`. Use `CryptoKey` (the actual
// return type of `importSPKI`) instead.
import { jwtVerify, importSPKI, type CryptoKey } from 'jose';

let cachedKey: CryptoKey | null = null;

export function resetKeyCache(): void {
  cachedKey = null;
}

async function getPublicKey(): Promise<CryptoKey> {
  if (cachedKey) return cachedKey;
  const pem = process.env.LICENSE_PUBLIC_KEY;
  if (!pem) {
    throw new Error('LICENSE_PUBLIC_KEY environment variable is required');
  }
  cachedKey = await importSPKI(pem, 'ES256');
  return cachedKey;
}

export interface VerifyResult {
  valid: boolean;
}

export async function verifyLicense(token: string): Promise<VerifyResult> {
  try {
    const publicKey = await getPublicKey();
    await jwtVerify(token, publicKey, { algorithms: ['ES256'] });
    return { valid: true };
  } catch {
    return { valid: false };
  }
}
```

- [ ] **Step 5: Run test to verify it passes**

```bash
pnpm test lib/jwt.test.ts
```

Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add apps/docs-shell/lib/jwt.ts apps/docs-shell/lib/jwt.test.ts
git commit -m "feat(docs-shell): add JWT verification helper with key cache"
```

---

## Task 6 — `lib/bauth.ts` (BAUTH callback fetch)

Thin wrapper around the BAUTH API. Encapsulates the pending/failure states so the callback route stays readable.

**Files:**
- Create: `apps/docs-shell/lib/bauth.ts`
- Test: `apps/docs-shell/lib/bauth.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// apps/docs-shell/lib/bauth.test.ts
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { exchangeGithubCode } from './bauth';

const originalFetch = globalThis.fetch;

describe('exchangeGithubCode', () => {
  beforeEach(() => {
    process.env.BAUTH_API_URL = 'https://api.test.example';
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('returns ok with license on 200', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ license: 'jwt.here' }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    ) as typeof fetch;

    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('ok');
    if (result.status === 'ok') expect(result.license).toBe('jwt.here');
  });

  it('returns pending on 403', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response('', { status: 403 })) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('pending');
  });

  it('returns error on 500', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response('', { status: 500 })) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('auth_failed');
  });

  it('returns error when body is missing license', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ wrong: 'shape' }), { status: 200 })
    ) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('invalid_response');
  });

  it('returns error when fetch throws', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('network down')) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('api_error');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test lib/bauth.test.ts
```

Expected: FAIL with "Cannot find module './bauth'".

- [ ] **Step 3: Write implementation**

```ts
// apps/docs-shell/lib/bauth.ts
export type ExchangeResult =
  | { status: 'ok'; license: string }
  | { status: 'pending' }
  | { status: 'error'; reason: 'api_error' | 'auth_failed' | 'invalid_response' };

function getApiUrl(): string {
  return process.env.BAUTH_API_URL ?? 'https://api.eddacraft.ai';
}

export async function exchangeGithubCode(code: string): Promise<ExchangeResult> {
  const url = `${getApiUrl()}/api/v1/auth/github/callback`;
  let res: Response;
  try {
    res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
  } catch {
    return { status: 'error', reason: 'api_error' };
  }

  if (res.status === 403) return { status: 'pending' };
  if (!res.ok) return { status: 'error', reason: 'auth_failed' };

  let body: unknown;
  try {
    body = await res.json();
  } catch {
    return { status: 'error', reason: 'invalid_response' };
  }

  if (!body || typeof (body as { license?: unknown }).license !== 'string') {
    return { status: 'error', reason: 'invalid_response' };
  }

  return { status: 'ok', license: (body as { license: string }).license };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test lib/bauth.test.ts
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/docs-shell/lib/bauth.ts apps/docs-shell/lib/bauth.test.ts
git commit -m "feat(docs-shell): add BAUTH exchange wrapper"
```

---

## Task 7 — `middleware.ts` (JWT gate for `/anvil/:path*`)

The critical piece. Runs on the edge, reads the `anvil-docs-session` cookie, verifies the JWT, and either passes through (letting the rewrite to `anvil-docs-private` fire) or redirects to `/auth/login`.

**Files:**
- Create: `apps/docs-shell/middleware.ts`
- Test: `apps/docs-shell/middleware.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// apps/docs-shell/middleware.test.ts
import { describe, it, expect, beforeAll, beforeEach, vi } from 'vitest';
import { SignJWT, importPKCS8 } from 'jose';
import middleware from './middleware';
import { resetKeyCache } from './lib/jwt';

// Reuse the same test keys from lib/jwt.test.ts — extract to a shared fixture if duplicated.
const TEST_PUBLIC_KEY_PEM = `-----BEGIN PUBLIC KEY-----
...
-----END PUBLIC KEY-----`;

const TEST_PRIVATE_KEY_PEM = `-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----`;

async function signToken(expSecondsFromNow: number = 3600): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  return new SignJWT({ sub: 'test@example.com' })
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSecondsFromNow)
    .sign(privateKey);
}

function makeRequest(url: string, cookies: Record<string, string> = {}): Request {
  const cookieHeader = Object.entries(cookies).map(([k, v]) => `${k}=${v}`).join('; ');
  return new Request(url, {
    headers: cookieHeader ? { cookie: cookieHeader } : {},
  });
}

describe('middleware', () => {
  beforeAll(() => {
    process.env.LICENSE_PUBLIC_KEY = TEST_PUBLIC_KEY_PEM;
  });

  beforeEach(() => {
    resetKeyCache();
  });

  it('redirects to login when no cookie', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview');
    const res = await middleware(req);
    expect(res.status).toBe(302);
    const location = res.headers.get('location')!;
    expect(location).toContain('/auth/login');
    expect(location).toContain('next=%2Fanvil%2Foverview');
  });

  it('passes through with a valid cookie', async () => {
    const token = await signToken();
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await middleware(req);
    // NextResponse.next() has a specific header signature — easier to check it is not a redirect
    expect(res.status).not.toBe(302);
  });

  it('redirects and clears cookie when token is expired', async () => {
    const token = await signToken(-60);
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await middleware(req);
    expect(res.status).toBe(302);
    expect(res.headers.get('set-cookie') ?? '').toContain('anvil-docs-session=');
    expect(res.headers.get('set-cookie') ?? '').toContain('Max-Age=0');
  });

  it('redirects when token is garbage', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': 'not.a.jwt',
    });
    const res = await middleware(req);
    expect(res.status).toBe(302);
  });

  it('preserves deep path in next param', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/quickstart/setup');
    const res = await middleware(req);
    expect(res.headers.get('location')).toContain('next=%2Fanvil%2Fquickstart%2Fsetup');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test middleware.test.ts
```

Expected: FAIL — middleware module does not exist.

- [ ] **Step 3: Write middleware.ts**

```ts
// apps/docs-shell/middleware.ts
import { NextResponse, type NextRequest } from 'next/server';
import { verifyLicense } from './lib/jwt';
import { getCookie } from './lib/cookie';

const COOKIE_NAME = 'anvil-docs-session';

function redirectToLogin(request: NextRequest | Request, clearCookie: boolean): NextResponse {
  const url = new URL(request.url);
  const loginUrl = new URL('/auth/login', url.origin);
  loginUrl.searchParams.set('next', url.pathname);
  const response = NextResponse.redirect(loginUrl, 302);
  if (clearCookie) {
    response.cookies.set({
      name: COOKIE_NAME,
      value: '',
      path: '/',
      maxAge: 0,
      httpOnly: true,
      secure: true,
      sameSite: 'lax',
    });
  }
  return response;
}

export default async function middleware(request: NextRequest | Request): Promise<NextResponse> {
  const cookieHeader = request.headers.get('cookie');
  const token = getCookie(cookieHeader, COOKIE_NAME);

  if (!token) return redirectToLogin(request, false);

  const { valid } = await verifyLicense(token);
  if (!valid) return redirectToLogin(request, true);

  return NextResponse.next();
}

export const config = {
  matcher: ['/anvil/:path*'],
};
```

- [ ] **Step 4: Run tests**

```bash
pnpm test middleware.test.ts
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/docs-shell/middleware.ts apps/docs-shell/middleware.test.ts
git commit -m "feat(docs-shell): add /anvil JWT gate middleware"
```

---

## Task 8 — `app/auth/login/route.ts`

**Files:**
- Create: `apps/docs-shell/app/auth/login/route.ts`

- [ ] **Step 1: Write the route**

```ts
// apps/docs-shell/app/auth/login/route.ts
import { NextResponse, type NextRequest } from 'next/server';
import { encryptState } from '@/lib/state';
import { validateNext } from '@/lib/next-url';

export const runtime = 'nodejs';

const GITHUB_AUTHORIZE_URL = 'https://github.com/login/oauth/authorize';
const SCOPES = 'read:user user:email';

function requireEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is required`);
  return v;
}

function randomNonce(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const next = validateNext(url.searchParams.get('next'));
  const nonce = randomNonce();

  const state = await encryptState({ next, nonce }, requireEnv('DOCS_STATE_SECRET'));

  const callbackUrl = new URL('/auth/callback', url.origin).toString();
  const authorizeUrl = new URL(GITHUB_AUTHORIZE_URL);
  authorizeUrl.searchParams.set('client_id', requireEnv('GITHUB_CLIENT_ID'));
  authorizeUrl.searchParams.set('redirect_uri', callbackUrl);
  authorizeUrl.searchParams.set('scope', SCOPES);
  authorizeUrl.searchParams.set('state', state);

  const response = NextResponse.redirect(authorizeUrl.toString(), 302);
  response.cookies.set({
    name: 'oauth-nonce',
    value: nonce,
    path: '/auth/callback',
    maxAge: 600,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm nx typecheck docs-shell
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/auth/login/route.ts
git commit -m "feat(docs-shell): add /auth/login route"
```

---

## Task 9 — `app/auth/callback/route.ts`

**Files:**
- Create: `apps/docs-shell/app/auth/callback/route.ts`

- [ ] **Step 1: Write the route**

```ts
// apps/docs-shell/app/auth/callback/route.ts
import { NextResponse, type NextRequest } from 'next/server';
import { decryptState } from '@/lib/state';
import { validateNext } from '@/lib/next-url';
import { exchangeGithubCode } from '@/lib/bauth';

export const runtime = 'nodejs';

const COOKIE_NAME = 'anvil-docs-session';
const COOKIE_MAX_AGE = 7 * 24 * 60 * 60;

function requireEnv(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is required`);
  return v;
}

function errorRedirect(origin: string, reason: string): NextResponse {
  const url = new URL('/auth/error', origin);
  url.searchParams.set('reason', reason);
  return NextResponse.redirect(url, 302);
}

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const code = url.searchParams.get('code');
  const stateParam = url.searchParams.get('state');
  const error = url.searchParams.get('error');

  if (error) {
    return errorRedirect(url.origin, error === 'access_denied' ? 'denied' : 'oauth_error');
  }

  if (!code || !stateParam) {
    return errorRedirect(url.origin, 'missing_params');
  }

  const state = await decryptState(stateParam, requireEnv('DOCS_STATE_SECRET'));
  if (!state) {
    return errorRedirect(url.origin, 'invalid_state');
  }

  const cookieNonce = request.cookies.get('oauth-nonce')?.value;
  if (!cookieNonce || cookieNonce !== state.nonce) {
    return errorRedirect(url.origin, 'csrf_mismatch');
  }

  const next = validateNext(state.next);

  const result = await exchangeGithubCode(code);
  if (result.status === 'pending') {
    return NextResponse.redirect(new URL('/auth/pending', url.origin), 302);
  }
  if (result.status === 'error') {
    return errorRedirect(url.origin, result.reason);
  }

  const response = NextResponse.redirect(new URL(next, url.origin), 302);
  response.cookies.set({
    name: COOKIE_NAME,
    value: result.license,
    path: '/',
    maxAge: COOKIE_MAX_AGE,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  response.cookies.set({
    name: 'oauth-nonce',
    value: '',
    path: '/auth/callback',
    maxAge: 0,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm nx typecheck docs-shell
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/auth/callback/route.ts
git commit -m "feat(docs-shell): add /auth/callback route"
```

---

## Task 10 — `app/auth/logout/route.ts`

**Files:**
- Create: `apps/docs-shell/app/auth/logout/route.ts`

- [ ] **Step 1: Write the route**

```ts
// apps/docs-shell/app/auth/logout/route.ts
import { NextResponse, type NextRequest } from 'next/server';

export const runtime = 'nodejs';

const COOKIE_NAME = 'anvil-docs-session';

export async function GET(request: NextRequest) {
  const url = new URL(request.url);
  const response = NextResponse.redirect(new URL('/', url.origin), 302);
  response.cookies.set({
    name: COOKIE_NAME,
    value: '',
    path: '/',
    maxAge: 0,
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
  });
  return response;
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm nx typecheck docs-shell
```

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/auth/logout/route.ts
git commit -m "feat(docs-shell): add /auth/logout route"
```

---

## Task 11 — `/auth/pending` and `/auth/error` pages

**Files:**
- Create: `apps/docs-shell/app/auth/pending/page.tsx`
- Create: `apps/docs-shell/app/auth/error/page.tsx`

- [ ] **Step 1: Write pending page**

```tsx
// apps/docs-shell/app/auth/pending/page.tsx
export const metadata = { title: 'Access Pending — EddaCraft Docs' };

export default function PendingPage() {
  return (
    <main
      style={{
        fontFamily: 'system-ui, sans-serif',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        margin: 0,
        background: '#0a0a0a',
        color: '#e5e5e5',
      }}
    >
      <div style={{ textAlign: 'center', maxWidth: 480, padding: '2rem' }}>
        <h1>Access Pending</h1>
        <p>
          Your GitHub account has been registered, but access to Anvil documentation requires
          approval.
        </p>
        <p>You&apos;ll receive an email once your access has been approved.</p>
        <p>
          <a href="/" style={{ color: '#60a5fa' }}>
            Return to docs home
          </a>
        </p>
      </div>
    </main>
  );
}
```

- [ ] **Step 2: Write error page**

```tsx
// apps/docs-shell/app/auth/error/page.tsx
export const metadata = { title: 'Sign-in error — EddaCraft Docs' };

const REASONS: Record<string, string> = {
  denied: 'You cancelled the GitHub sign-in.',
  oauth_error: 'GitHub returned an OAuth error.',
  missing_params: 'The callback URL is missing required parameters.',
  invalid_state: 'The OAuth state parameter was invalid or tampered with.',
  csrf_mismatch: 'CSRF nonce did not match. Please try signing in again.',
  api_error: 'Could not reach the authentication service.',
  auth_failed: 'Authentication failed.',
  invalid_response: 'The authentication service returned an unexpected response.',
};

export default async function ErrorPage({
  searchParams,
}: {
  searchParams: Promise<{ reason?: string }>;
}) {
  const { reason } = await searchParams;
  const message = (reason && REASONS[reason]) ?? 'An unknown error occurred.';

  return (
    <main
      style={{
        fontFamily: 'system-ui, sans-serif',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        margin: 0,
        background: '#0a0a0a',
        color: '#e5e5e5',
      }}
    >
      <div style={{ textAlign: 'center', maxWidth: 480, padding: '2rem' }}>
        <h1>Sign-in error</h1>
        <p>{message}</p>
        <p>
          <a href="/auth/login" style={{ color: '#60a5fa', marginRight: '1rem' }}>
            Try again
          </a>
          <a href="/" style={{ color: '#60a5fa' }}>
            Return home
          </a>
        </p>
      </div>
    </main>
  );
}
```

- [ ] **Step 3: Typecheck**

```bash
pnpm nx typecheck docs-shell
```

- [ ] **Step 4: Commit**

```bash
git add apps/docs-shell/app/auth/pending/page.tsx apps/docs-shell/app/auth/error/page.tsx
git commit -m "feat(docs-shell): add /auth/pending and /auth/error pages"
```

---

## Task 12 — `app/layout.tsx` and `app/page.tsx` (landing)

Ports the existing `apps/docs-site/src/pages/index.tsx` content into the shell as the root page. The Docusaurus-specific `<Link to=...>` components become plain `<a href=...>`.

**Files:**
- Create: `apps/docs-shell/app/layout.tsx`
- Create: `apps/docs-shell/app/page.tsx`
- Create: `apps/docs-shell/app/globals.css`

- [ ] **Step 1: Read the current homepage for reference**

```bash
cat apps/docs-site/src/pages/index.tsx
```

Note the hero copy, section structure, and CTA destinations. The three primary CTAs point to `/anvil/overview`, `/kindling/overview`, `/aps/overview`.

- [ ] **Step 2: Write globals.css**

```css
/* apps/docs-shell/app/globals.css */
:root {
  --bg: #0a0a0a;
  --fg: #e5e5e5;
  --accent: #2563eb;
  --accent-hover: #1d4ed8;
  --muted: #a3a3a3;
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  background: var(--bg);
  color: var(--fg);
  font-family: system-ui, -apple-system, sans-serif;
  line-height: 1.6;
}

a { color: inherit; }

.hero {
  padding: 6rem 2rem 4rem;
  text-align: center;
  max-width: 960px;
  margin: 0 auto;
}

.hero h1 {
  font-size: clamp(2rem, 5vw, 3.5rem);
  margin: 0 0 1rem;
}

.hero p.tagline {
  font-size: 1.25rem;
  color: var(--muted);
  margin: 0 0 2rem;
}

.cta {
  display: inline-block;
  padding: 0.75rem 1.5rem;
  background: var(--accent);
  color: white;
  text-decoration: none;
  border-radius: 0.5rem;
  font-weight: 500;
  margin: 0.5rem;
}

.cta:hover { background: var(--accent-hover); }

.cta.secondary {
  background: transparent;
  border: 1px solid var(--accent);
  color: var(--accent);
}

.sections {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1.5rem;
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto 4rem;
}

.section {
  padding: 1.5rem;
  border: 1px solid #262626;
  border-radius: 0.75rem;
  background: #111;
}

.section h3 { margin-top: 0; }
.section a { color: #60a5fa; }
```

- [ ] **Step 3: Write layout**

```tsx
// apps/docs-shell/app/layout.tsx
import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'EddaCraft Docs',
  description: 'The forge for governed AI-assisted work',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
```

- [ ] **Step 4: Write landing page**

```tsx
// apps/docs-shell/app/page.tsx
export default function HomePage() {
  return (
    <>
      <section className="hero">
        <h1>EddaCraft</h1>
        <p className="tagline">The forge for governed AI-assisted work.</p>
        <a className="cta" href="/anvil/overview">
          Anvil docs
        </a>
        <a className="cta secondary" href="/aps/overview">
          APS spec
        </a>
      </section>

      <section className="sections">
        <div className="section">
          <h3>Anvil</h3>
          <p>Commercial beta: governed code-gen pipelines for engineering teams.</p>
          <a href="/anvil/overview">Read the Anvil docs →</a>
        </div>
        <div className="section">
          <h3>APS</h3>
          <p>Open-source Anvil Plan Spec: declarative implementation plans.</p>
          <a href="/aps/overview">Read the APS spec →</a>
        </div>
        <div className="section">
          <h3>Kindling</h3>
          <p>Open-source observation capture and memory substrate.</p>
          <a href="/kindling/overview">Read the Kindling docs →</a>
        </div>
        <div className="section">
          <h3>edda-stack</h3>
          <p>Open-source integration layer between Anvil, APS, and Kindling.</p>
          <a href="/edda-stack/overview">Read the edda-stack docs →</a>
        </div>
      </section>
    </>
  );
}
```

- [ ] **Step 5: Typecheck**

```bash
pnpm nx typecheck docs-shell
```

- [ ] **Step 6: Commit**

```bash
git add apps/docs-shell/app/layout.tsx apps/docs-shell/app/page.tsx apps/docs-shell/app/globals.css
git commit -m "feat(docs-shell): add landing page and root layout"
```

---

## Task 13 — `/robots.txt` and `/llms.txt` routes

**Files:**
- Create: `apps/docs-shell/app/robots.txt/route.ts`
- Create: `apps/docs-shell/app/llms.txt/route.ts`

- [ ] **Step 1: Write robots.txt route**

```ts
// apps/docs-shell/app/robots.txt/route.ts
export const runtime = 'nodejs';

const BODY = `User-agent: *
Disallow: /anvil/
Disallow: /auth/

Sitemap: https://docs.eddacraft.ai/sitemap.xml
`;

export async function GET() {
  return new Response(BODY, {
    status: 200,
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
```

- [ ] **Step 2: Write llms.txt route**

```ts
// apps/docs-shell/app/llms.txt/route.ts
export const runtime = 'nodejs';

const BODY = `# EddaCraft Documentation
# Anvil is a commercial product in closed beta. Anvil documentation is private.
# Public sections: /kindling, /aps, /edda-stack, /blog

User-agent: *
Disallow: /anvil/
`;

export async function GET() {
  return new Response(BODY, {
    status: 200,
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
```

- [ ] **Step 3: Commit**

```bash
git add apps/docs-shell/app/robots.txt/route.ts apps/docs-shell/app/llms.txt/route.ts
git commit -m "feat(docs-shell): add robots.txt and llms.txt"
```

---

## Task 14 — `next.config.ts` and `vercel.json` (shell rewrites, placeholder destinations)

Use placeholder Vercel preview URLs for now. Real URLs are filled in once upstream projects are deployed (Task 20).

**Files:**
- Create: `apps/docs-shell/next.config.ts`
- Create: `apps/docs-shell/vercel.json`

- [ ] **Step 1: Write next.config.ts**

```ts
// apps/docs-shell/next.config.ts
import type { NextConfig } from 'next';

const ANVIL_DOCS_URL = process.env.ANVIL_DOCS_URL ?? 'https://anvil-docs-private.vercel.app';
const PUBLIC_DOCS_URL = process.env.PUBLIC_DOCS_URL ?? 'https://docs-public.vercel.app';

const config: NextConfig = {
  async rewrites() {
    return [
      { source: '/anvil/:path*', destination: `${ANVIL_DOCS_URL}/anvil/:path*` },
      { source: '/kindling/:path*', destination: `${PUBLIC_DOCS_URL}/kindling/:path*` },
      { source: '/aps/:path*', destination: `${PUBLIC_DOCS_URL}/aps/:path*` },
      { source: '/edda-stack/:path*', destination: `${PUBLIC_DOCS_URL}/edda-stack/:path*` },
      { source: '/blog/:path*', destination: `${PUBLIC_DOCS_URL}/blog/:path*` },
    ];
  },
  async headers() {
    return [
      {
        source: '/anvil/:path*',
        headers: [{ key: 'X-Robots-Tag', value: 'noindex, nofollow' }],
      },
    ];
  },
};

export default config;
```

- [ ] **Step 2: Write vercel.json**

```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "framework": "nextjs",
  "buildCommand": "pnpm nx build docs-shell",
  "installCommand": "pnpm install --frozen-lockfile",
  "outputDirectory": ".next"
}
```

- [ ] **Step 3: Typecheck and lint**

```bash
pnpm nx typecheck docs-shell
pnpm nx lint docs-shell
```

- [ ] **Step 4: Commit**

```bash
git add apps/docs-shell/next.config.ts apps/docs-shell/vercel.json
git commit -m "feat(docs-shell): wire rewrites and vercel config"
```

---

## Task 15 — Deploy docs-shell to Vercel preview and smoke-test

**Files:** None (deployment)

- [ ] **Step 1: Create the Vercel project via CLI**

```bash
cd apps/docs-shell
vercel link --project=eddacraft-docs-shell --yes
```

- [ ] **Step 2: Set required env vars on the preview environment**

```bash
vercel env add LICENSE_PUBLIC_KEY preview
vercel env add DOCS_STATE_SECRET preview
vercel env add GITHUB_CLIENT_ID preview
vercel env add GITHUB_CLIENT_SECRET preview
vercel env add BAUTH_API_URL preview  # value: https://api.eddacraft.ai
```

Get `LICENSE_PUBLIC_KEY` and `DOCS_STATE_SECRET` from Azure Key Vault `kv-iac-anvil` (they already exist from DOCSAUTH-004).

- [ ] **Step 3: Deploy to preview**

```bash
vercel deploy
```

Record the preview URL.

- [ ] **Step 4: Smoke test unauthenticated flow**

```bash
PREVIEW_URL="https://eddacraft-docs-shell-<hash>.vercel.app"

# Landing page — expect 200
curl -s -o /dev/null -w "/ %{http_code}\n" "$PREVIEW_URL/"

# /anvil/overview without cookie — expect 302 to /auth/login
curl -s -o /dev/null -w "/anvil/overview %{http_code} %{redirect_url}\n" "$PREVIEW_URL/anvil/overview"

# /auth/login — expect 200 HTML with GitHub authorize link
curl -s "$PREVIEW_URL/auth/login" | grep -o 'github.com/login/oauth/authorize' | head -1

# /robots.txt — expect 200 with Disallow: /anvil/
curl -s "$PREVIEW_URL/robots.txt"
```

Expected:
- `/` → 200
- `/anvil/overview` → 302 with redirect_url containing `/auth/login?next=%2Fanvil%2Foverview`
- `/auth/login` → contains GitHub authorize URL
- `/robots.txt` → contains `Disallow: /anvil/`

- [ ] **Step 5: Commit**

No code changes in this task. If preview deployment requires a `vercel.json` tweak, amend Task 14 commit rather than creating a new one.

---

## Task 16 — Create `apps/anvil-docs-private` project (Anvil plugin only, `baseUrl: '/anvil/'`)

**Files:**
- Create: `apps/anvil-docs-private/package.json`
- Create: `apps/anvil-docs-private/tsconfig.json`
- Create: `apps/anvil-docs-private/project.json`
- Create: `apps/anvil-docs-private/vercel.json`
- Create: `apps/anvil-docs-private/docusaurus.config.ts`
- Create: `apps/anvil-docs-private/sidebars/anvil.ts`
- Create: `apps/anvil-docs-private/src/css/custom.css`
- Create: `apps/anvil-docs-private/static/img/favicon.svg`

- [ ] **Step 1: Copy minimal Docusaurus shell from docs-site**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
mkdir -p apps/anvil-docs-private/sidebars apps/anvil-docs-private/src/css apps/anvil-docs-private/static/img
cp apps/docs-site/sidebars/anvil.ts apps/anvil-docs-private/sidebars/anvil.ts
cp apps/docs-site/src/css/custom.css apps/anvil-docs-private/src/css/custom.css
cp apps/docs-site/static/img/favicon.svg apps/anvil-docs-private/static/img/favicon.svg
cp apps/docs-site/tsconfig.json apps/anvil-docs-private/tsconfig.json
```

- [ ] **Step 2: Write package.json**

```json
{
  "name": "@eddacraft/anvil-docs-private",
  "version": "0.3.0-beta",
  "private": true,
  "scripts": {
    "docusaurus": "docusaurus",
    "start": "docusaurus start --port 3101",
    "build": "docusaurus build",
    "clear": "docusaurus clear",
    "serve": "docusaurus serve",
    "typecheck": "tsc"
  },
  "dependencies": {
    "@docusaurus/preset-classic": "3.10.0",
    "@docusaurus/faster": "3.10.0",
    "@docusaurus/plugin-content-docs": "3.10.0",
    "@docusaurus/types": "3.10.0",
    "clsx": "^2.1.1",
    "prism-react-renderer": "^2.4.1",
    "react": "19.2.4",
    "react-dom": "19.2.4"
  },
  "devDependencies": {
    "@docusaurus/module-type-aliases": "3.10.0",
    "@docusaurus/tsconfig": "3.10.0",
    "typescript": "~6.0.2"
  }
}
```

- [ ] **Step 3: Write docusaurus.config.ts**

```ts
import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Anvil',
  tagline: 'Governed AI-assisted work',
  favicon: 'img/favicon.svg',

  future: { v4: true },

  url: 'https://docs.eddacraft.ai',
  baseUrl: '/anvil/',

  organizationName: 'EddaCraft',
  projectName: 'anvil-docs',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  markdown: { format: 'detect' },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: false,
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'anvil',
        path: '../../docs/public/anvil',
        routeBasePath: '/',
        sidebarPath: './sidebars/anvil.ts',
        editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/docs/public/anvil/',
      },
    ],
  ],

  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Anvil Docs',
      items: [
        { href: 'https://docs.eddacraft.ai/', label: 'Back to EddaCraft', position: 'right' },
        { href: '/auth/logout', label: 'Sign out', position: 'right' },
      ],
    },
    prism: {
      theme: prismThemes.vsDark,
      darkTheme: prismThemes.vsDark,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
```

Note: `routeBasePath: '/'` combined with `baseUrl: '/anvil/'` produces effective routes under `/anvil/*`.

- [ ] **Step 4: Write project.json**

```json
{
  "name": "anvil-docs-private",
  "$schema": "../../node_modules/nx/schemas/project-schema.json",
  "sourceRoot": "apps/anvil-docs-private",
  "projectType": "application",
  "targets": {
    "build": { "executor": "nx:run-script", "options": { "script": "build" } },
    "start": { "executor": "nx:run-script", "options": { "script": "start" } },
    "serve": { "executor": "nx:run-script", "options": { "script": "serve" } },
    "typecheck": { "executor": "nx:run-script", "options": { "script": "typecheck" } }
  },
  "tags": ["app", "docs"]
}
```

- [ ] **Step 5: Write vercel.json**

```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "buildCommand": "pnpm nx build anvil-docs-private",
  "installCommand": "pnpm install --frozen-lockfile",
  "outputDirectory": "build"
}
```

- [ ] **Step 6: Install and build locally**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
pnpm install
pnpm nx build anvil-docs-private
```

Expected: build succeeds. Output in `apps/anvil-docs-private/build/anvil/`.

- [ ] **Step 7: Verify baseUrl assumption held (repeat of Task 0 check, on the real build)**

```bash
find apps/anvil-docs-private/build -type f -not -path "*/anvil/*" | grep -v -E "\.(html|txt|xml|ico)$" || echo CLEAN
grep -o 'src="[^"]*"' apps/anvil-docs-private/build/anvil/index.html | head -5
```

Expected: `CLEAN`, all asset srcs under `/anvil/assets/`.

- [ ] **Step 8: Commit**

```bash
git add apps/anvil-docs-private pnpm-lock.yaml
git commit -m "feat(anvil-docs-private): split Anvil docs into gated build with /anvil/ baseUrl"
```

---

## Task 17 — Deploy anvil-docs-private to Vercel preview

**Files:** None

- [ ] **Step 1: Link and deploy**

```bash
cd apps/anvil-docs-private
vercel link --project=eddacraft-anvil-docs-private --yes
vercel deploy
```

Record the preview URL.

- [ ] **Step 2: Smoke test direct access**

```bash
PRIVATE_URL="https://eddacraft-anvil-docs-private-<hash>.vercel.app"

curl -s -o /dev/null -w "%{http_code}\n" "$PRIVATE_URL/anvil/"
curl -s "$PRIVATE_URL/anvil/" | grep -o '/anvil/assets/js/[^"]*\.js' | head -3
```

Expected: 200 on `/anvil/`, all chunk URLs under `/anvil/assets/js/`.

- [ ] **Step 3: Record preview URL for shell rewrite config**

Note the URL. You'll use it as `ANVIL_DOCS_URL` in Task 20.

---

## Task 18 — Create `apps/docs-public` from `apps/docs-site` (pruned)

Rather than renaming in place, copy to the new name and prune; retain `docs-site` untouched until the cutover commit so rollback remains trivial.

**Files:**
- Create: `apps/docs-public/` (copied from docs-site)
- Modify: `apps/docs-public/docusaurus.config.ts` — prune anvil, beta, start-here plugins
- Modify: `apps/docs-public/package.json` — rename
- Delete: `apps/docs-public/src/pages/index.tsx`
- Delete: `apps/docs-public/middleware.ts`
- Delete: `apps/docs-public/api/`
- Delete: `apps/docs-public/TOGGLING-DOCS.md`

- [ ] **Step 1: Copy docs-site to docs-public**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
cp -r apps/docs-site apps/docs-public
rm -rf apps/docs-public/node_modules apps/docs-public/.docusaurus apps/docs-public/build
```

- [ ] **Step 2: Rename package**

Edit `apps/docs-public/package.json`:
- Change `"name": "@eddacraft/docs-site"` to `"name": "@eddacraft/docs-public"`
- Change `"start": "docusaurus start"` to `"start": "docusaurus start --port 3102"`

Edit `apps/docs-public/project.json`:
- Change `"name": "docs-site"` to `"name": "docs-public"`
- Change `"sourceRoot": "apps/docs-site"` to `"sourceRoot": "apps/docs-public"`

- [ ] **Step 3: Prune the Docusaurus config**

Edit `apps/docs-public/docusaurus.config.ts`:
- Remove the `anvil` plugin entry (lines with `id: 'anvil'`)
- Remove the `beta` plugin entry (lines with `id: 'beta'`)
- Remove the commented-out `start-here` block entirely
- Keep `aps`, `kindling`, `edda-stack` plugins
- Leave `baseUrl: '/'` as-is

Verify remaining plugins:

```bash
grep -A1 "id: '" apps/docs-public/docusaurus.config.ts
```

Expected: only `id: 'aps'`, `id: 'kindling'`, `id: 'edda-stack'`.

- [ ] **Step 4: Delete migrated files**

```bash
rm apps/docs-public/src/pages/index.tsx
rm apps/docs-public/src/pages/index.module.css 2>/dev/null || true
rm apps/docs-public/middleware.ts
rm -rf apps/docs-public/api
rm apps/docs-public/TOGGLING-DOCS.md
```

- [ ] **Step 5: Remove jose dep and middleware function config**

Edit `apps/docs-public/package.json`: remove the `"jose": "^6.2.2"` line from dependencies.

Edit `apps/docs-public/vercel.json`: remove any `functions` or `rewrites` config related to middleware and auth; it should reduce to a minimal Docusaurus build config.

- [ ] **Step 6: Install and build**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
pnpm install
pnpm nx build docs-public
```

Expected: build succeeds; output contains `build/kindling/`, `build/aps/`, `build/edda-stack/`, but **no** `build/anvil/` or `build/beta/`.

Verify:

```bash
ls apps/docs-public/build/
# Expected: aps, blog, edda-stack, kindling, index.html, 404.html, etc.
# NOT expected: anvil, beta
```

- [ ] **Step 7: Commit**

```bash
git add apps/docs-public pnpm-lock.yaml
git commit -m "feat(docs-public): split public docs build (no Anvil, no beta)"
```

---

## Task 19 — Deploy docs-public to Vercel preview

**Files:** None

- [ ] **Step 1: Link and deploy**

```bash
cd apps/docs-public
vercel link --project=eddacraft-docs-public --yes
vercel deploy
```

Record the preview URL.

- [ ] **Step 2: Smoke test**

```bash
PUBLIC_URL="https://eddacraft-docs-public-<hash>.vercel.app"

curl -s -o /dev/null -w "/kindling/ %{http_code}\n" "$PUBLIC_URL/kindling/"
curl -s -o /dev/null -w "/aps/ %{http_code}\n" "$PUBLIC_URL/aps/"
curl -s -o /dev/null -w "/anvil/ %{http_code}\n" "$PUBLIC_URL/anvil/"
# Expected: kindling 200, aps 200, anvil 404 (no Anvil plugin in this build)
```

---

## Task 20 — Wire shell rewrites to real preview URLs

**Files:**
- Modify: `apps/docs-shell/next.config.ts` (use env vars, already done) — set env in Vercel

- [ ] **Step 1: Set env vars on the shell preview environment**

```bash
cd apps/docs-shell
vercel env add ANVIL_DOCS_URL preview
# Paste the anvil-docs-private preview URL from Task 17
vercel env add PUBLIC_DOCS_URL preview
# Paste the docs-public preview URL from Task 19
```

- [ ] **Step 2: Redeploy shell to pick up env vars**

```bash
vercel deploy
```

- [ ] **Step 3: Full-flow smoke test**

```bash
SHELL_URL="https://eddacraft-docs-shell-<hash>.vercel.app"

# Public rewrite
curl -s -o /dev/null -w "/kindling/ %{http_code}\n" "$SHELL_URL/kindling/"

# Anvil path unauthenticated → middleware 302
curl -s -o /dev/null -w "/anvil/overview %{http_code}\n" "$SHELL_URL/anvil/overview"

# Anvil chunk unauthenticated → middleware 302 (the critical assertion)
# First get a chunk URL from the deployed build
CHUNK=$(curl -s "$SHELL_URL/anvil/" 2>/dev/null | grep -oE '/anvil/assets/js/[^"]+\.js' | head -1)
echo "Chunk: $CHUNK"
curl -s -o /dev/null -w "chunk %{http_code}\n" "$SHELL_URL$CHUNK"
# Expected: 302 (middleware gated the chunk too)
```

If the chunk returns 200, the whole fix failed — investigate the middleware matcher.

- [ ] **Step 4: Manual browser flow test**

Open the shell preview URL in a browser. Click the "Anvil docs" CTA. Verify the browser redirects to GitHub, then back through `/auth/callback`, then lands on `/anvil/overview` with content visible. Sign out and verify the session is cleared.

- [ ] **Step 5: No commit yet** — everything is env var configuration at this stage.

---

## Task 21 — Pulumi: add the two new Vercel projects

**Files:**
- Modify: `infra/src/vercel.ts`

- [ ] **Step 1: Read existing vercel.ts structure**

```bash
cat infra/src/vercel.ts | head -60
```

Note the existing project-creation pattern. The current `docs-site` project resource is the reference for the new ones.

Use the existing `VercelApp` component (`infra/src/components/vercel-app.ts`)
— do NOT drop down to raw `@pulumiverse/vercel` resources. `VercelApp` handles
the shared ignore-command logic and marks all env vars as sensitive by
default, matching the rest of the monorepo.

Upstream origin wiring: `VercelApp` exposes `projectId` and `domainNames`
— it does **not** expose a `productionUrl`. Wire the shell to upstream
Docusaurus projects by passing the known domain strings as explicit config
values (the domains you registered on `anvilDocsPrivateApp` and
`docsPublicApp` via the `domains:` field), not Pulumi outputs. Declare the
two Docusaurus apps **before** the shell so the domain strings are in scope.

- [ ] **Step 2: Add `anvil-docs-private` app (declare first)**

```ts
// infra/src/vercel.ts
import { VercelApp } from './components/vercel-app';

const ANVIL_DOCS_PRIVATE_DOMAIN = 'anvil-docs-private.vercel.app';

const anvilDocsPrivateApp = new VercelApp('anvil-docs-private', {
  name: 'eddacraft-anvil-docs-private',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/anvil-docs-private',
  gitRepo: 'EddaCraft/anvil-001',
  domains: [ANVIL_DOCS_PRIVATE_DOMAIN],
  buildCommand: 'pnpm nx build anvil-docs-private',
  installCommand: 'pnpm install --frozen-lockfile',
  skipPreviewDeploys: true,
});
```

- [ ] **Step 3: Add `docs-public` app**

```ts
const DOCS_PUBLIC_DOMAIN = 'docs-public.vercel.app';

const docsPublicApp = new VercelApp('docs-public', {
  name: 'eddacraft-docs-public',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-public',
  gitRepo: 'EddaCraft/anvil-001',
  domains: [DOCS_PUBLIC_DOMAIN],
  buildCommand: 'pnpm nx build docs-public',
  installCommand: 'pnpm install --frozen-lockfile',
  skipPreviewDeploys: true,
});
```

- [ ] **Step 4: Add `docs-shell` app (references the domains above)**

```ts
const docsShellApp = new VercelApp('docs-shell', {
  name: 'eddacraft-docs-shell',
  framework: 'nextjs',
  rootDirectory: 'apps/docs-shell',
  gitRepo: 'EddaCraft/anvil-001',
  domains: ['docs.eddacraft.ai'],
  buildCommand: 'pnpm nx build docs-shell',
  installCommand: 'pnpm install --frozen-lockfile',
  envVars: {
    LICENSE_PUBLIC_KEY: licensePublicKey,
    DOCS_STATE_SECRET: docsStateSecret,
    GITHUB_CLIENT_ID: githubClientId,
    GITHUB_CLIENT_SECRET: githubClientSecret,
    BAUTH_API_URL: 'https://api.eddacraft.ai',
    ANVIL_DOCS_URL: `https://${ANVIL_DOCS_PRIVATE_DOMAIN}`,
    PUBLIC_DOCS_URL: `https://${DOCS_PUBLIC_DOMAIN}`,
  },
});
```

- [ ] **Step 5: Preview the Pulumi change**

```bash
cd infra
pulumi preview
```

Expected: three `vercel.Project` creations, no updates to existing resources yet.

- [ ] **Step 6: Apply**

```bash
pulumi up
```

Expected: three projects created. Record their Vercel IDs from the output.

- [ ] **Step 7: Commit**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
git add infra/src/vercel.ts
git commit -m "feat(infra): add docs-shell, anvil-docs-private, docs-public Vercel projects"
```

---

## Task 22 — Pulumi: deployment protection on upstream projects

Prevents direct access to `eddacraft-anvil-docs-private.vercel.app` and `eddacraft-docs-public.vercel.app`. Users reach them only via shell rewrites.

**Files:**
- Modify: `infra/src/vercel.ts`

- [ ] **Step 1: Add deployment protection to both upstream projects**

If `@pulumiverse/vercel` exposes `vercelAuthentication`:

```ts
// on both anvilDocsPrivateProject and docsPublicProject:
vercelAuthentication: {
  deploymentType: 'all_deployments',  // gate all deployments including preview
},
```

If that field isn't available, use a `Project Bypass Token` approach — configure a secret header the shell attaches to rewrites.

- [ ] **Step 2: Fallback — if Deployment Protection isn't available on plan**

Add a shared secret header:
1. Add env var `UPSTREAM_SECRET` on all three projects (Pulumi resource env).
2. Modify `apps/docs-shell/next.config.ts` rewrites to include a `headers` array on each rewrite:

   ```ts
   // next.config.ts — rewrites section update
   // Note: Next.js `rewrites()` supports a `headers` field on each entry in Next 15+
   { source: '/anvil/:path*', destination: `${ANVIL_DOCS_URL}/anvil/:path*`, has: [] },
   ```

   Since Next.js `rewrites` does not natively forward custom headers at the config level, use a Vercel project-level route instead via `vercel.json` on the shell:

   ```json
   {
     "rewrites": [
       { "source": "/anvil/(.*)", "destination": "https://eddacraft-anvil-docs-private.vercel.app/anvil/$1", "headers": { "x-upstream-secret": "$UPSTREAM_SECRET" } }
     ]
   }
   ```

3. On each upstream, add a minimal edge middleware (or Docusaurus plugin) that returns 401 unless `x-upstream-secret` matches.

- [ ] **Step 3: Preview and apply**

```bash
cd infra
pulumi preview
pulumi up
```

- [ ] **Step 4: Verify direct access is blocked**

```bash
curl -s -o /dev/null -w "%{http_code}\n" "https://eddacraft-anvil-docs-private.vercel.app/anvil/overview"
# Expected: 401 or redirect to Vercel auth, not 200
```

- [ ] **Step 5: Commit**

```bash
git add infra/src/vercel.ts apps/docs-shell/vercel.json 2>/dev/null || git add infra/src/vercel.ts
git commit -m "feat(infra): lock upstream docs projects behind deployment protection"
```

---

## Task 23 — Update GitHub OAuth App callback URL (manual)

**Files:** None (external config)

- [ ] **Step 1: Open GitHub OAuth App settings**

Navigate to https://github.com/organizations/EddaCraft/settings/applications and select the Anvil Docs OAuth App.

- [ ] **Step 2: Update callback URLs**

- Add: `https://docs.eddacraft.ai/auth/callback` (if not already present)
- Add: `https://eddacraft-docs-shell-*.vercel.app/auth/callback` (preview deployments, use a wildcard if the OAuth App supports it; otherwise add a specific preview URL during testing and remove before production)

- [ ] **Step 3: Save**

No commit — document the change in a comment on the eventual cutover PR.

---

## Task 24 — Pulumi: domain cutover (`docs.eddacraft.ai` → `docs-shell`)

**Files:**
- Modify: `infra/src/vercel.ts`

**This is the one-way door.** After this runs, production traffic hits the shell. Rollback is reassigning the domain back to `docs-site` in the Vercel dashboard.

- [ ] **Step 1: Move the domain binding**

In `infra/src/vercel.ts`:

- Remove `docs.eddacraft.ai` from the `docs-site` (or `docs-public`) project domains.
- Add `docs.eddacraft.ai` to the `docs-shell` project domains.

```ts
new vercel.ProjectDomain('docs-shell-apex', {
  projectId: docsShellProject.id,
  domain: 'docs.eddacraft.ai',
});
```

- [ ] **Step 2: Preview the change**

```bash
cd infra
pulumi preview
```

Expected: one domain deletion on the old project, one creation on `docs-shell`.

- [ ] **Step 3: Apply, during a low-traffic window**

```bash
pulumi up
```

- [ ] **Step 4: Verify DNS resolution**

```bash
curl -s -o /dev/null -w "%{http_code}\n" https://docs.eddacraft.ai/
curl -s -o /dev/null -w "%{http_code}\n" https://docs.eddacraft.ai/anvil/overview
curl -s -o /dev/null -w "%{http_code}\n" https://docs.eddacraft.ai/kindling/
```

Expected: `/` 200 (landing), `/anvil/overview` 302 (login redirect), `/kindling/` 200 (rewrite).

- [ ] **Step 5: Commit**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
git add infra/src/vercel.ts
git commit -m "feat(infra): cutover docs.eddacraft.ai to docs-shell project"
```

---

## Task 25 — Production smoke test

**Files:** None

- [ ] **Step 1: Run the full auth flow end-to-end**

1. Open https://docs.eddacraft.ai/ in a private browser window.
2. Click the "Anvil docs" CTA — expect redirect to GitHub OAuth.
3. Sign in with the test alt (`aneki@eddacraft.ai`'s GitHub account).
4. Expect redirect back through `/auth/callback` and land on `/anvil/overview` with content visible.
5. Navigate via sidebar links within `/anvil/*`. Every page should render.
6. Visit `/auth/logout` — expect redirect to `/` and cookie cleared.
7. Visit `/anvil/overview` again — expect redirect back to `/auth/login`.

- [ ] **Step 2: Run the bypass-attempt checks**

```bash
# 1. Unauthenticated content fetch — must be 302, never 200
curl -s -o /dev/null -w "anvil overview: %{http_code}\n" https://docs.eddacraft.ai/anvil/overview

# 2. Unauthenticated chunk fetch — must be 302 (the SPA-bypass fix)
CHUNK=$(curl -sL https://docs.eddacraft.ai/auth/login 2>/dev/null | grep -oE '/anvil/assets/js/[^"]+\.js' | head -1)
# If that's empty, fetch /anvil/ with a mock cookie from a known-good session for the discovery, then:
echo "If chunk detected: $CHUNK"

# 3. Direct upstream access — must be blocked
curl -s -o /dev/null -w "direct anvil-docs-private: %{http_code}\n" https://eddacraft-anvil-docs-private.vercel.app/anvil/overview
curl -s -o /dev/null -w "direct docs-public: %{http_code}\n" https://eddacraft-docs-public.vercel.app/kindling/

# 4. robots.txt contains the right Disallow
curl -s https://docs.eddacraft.ai/robots.txt | grep Disallow

# 5. X-Robots-Tag header on /anvil/* responses
curl -sI https://docs.eddacraft.ai/anvil/overview | grep -i x-robots
```

Expected: item 1 is 302; item 3 is 401 or Vercel auth redirect; item 4 lists `/anvil/`; item 5 has `X-Robots-Tag: noindex, nofollow`.

- [ ] **Step 3: If anything fails, halt and investigate**

Rollback is Vercel-dashboard domain reassignment to `docs-site`. Do not proceed to Task 26 until smoke test is green.

---

## Task 26 — Retire `apps/docs-site`

Runs **after** Task 25 has been green for at least 24 hours in production.

**Files:**
- Delete: `apps/docs-site/`
- Modify: `infra/src/vercel.ts` — remove old `docs-site` Vercel project resource

- [ ] **Step 1: Delete the directory**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
git rm -r apps/docs-site
```

- [ ] **Step 2: Remove the Pulumi resource**

In `infra/src/vercel.ts`, delete the `docs-site` project declaration and any references.

- [ ] **Step 3: Preview Pulumi**

```bash
cd infra
pulumi preview
```

Expected: one `vercel.Project` deletion and any associated env vars.

- [ ] **Step 4: Apply**

```bash
pulumi up
```

- [ ] **Step 5: Commit**

```bash
cd /home/aneki/Projects/src/EddaCraft/anvil-001
git add apps/docs-site infra/src/vercel.ts
git commit -m "chore(docs-site): retire old single-build project"
```

---

## Task 27 — Create DOCSAUTH2 APS module

**Files:**
- Create: `plans/modules/docs-auth-spa-gap.aps.md`

- [ ] **Step 1: Read an existing APS module for format**

```bash
cat plans/modules/docs-auth-gating.aps.md
```

- [ ] **Step 2: Write the new APS module**

```markdown
<!--
APS Module: Docs Auth SPA Gap
==============================
Replace the broken single-build docs auth gate with a Next.js shell fronting
two separated Docusaurus builds.

Scopes: DOCSAUTH2 (main)
-->

# Docs Auth SPA Gap

| ID        | Owner | Status       |
| --------- | ----- | ------------ |
| DOCSAUTH2 | —     | In Progress  |

## Purpose

The existing DOCSAUTH middleware gates `/anvil/:path*` on `docs.eddacraft.ai`
but does not actually prevent unauthenticated access because Docusaurus is
an SPA — `<Link>` navigations use `history.pushState`, the route table ships
in `main.js`, and code-split chunks at `/assets/js/*` are reachable
unauthenticated. This module replaces the architecture with a Next.js shell
fronting two separated Docusaurus builds so that `baseUrl: '/anvil/'` on
the private build puts chunks under `/anvil/assets/*` where the middleware
matcher catches them.

**Spec:** `plans/specs/2026-04-11-docs-auth-spa-gap-design.md`
**Plan:** `plans/specs/2026-04-11-docs-auth-spa-gap-plan.md`

## In Scope

- New `apps/docs-shell` Next.js 16 project
- New `apps/anvil-docs-private` Docusaurus build (Anvil only, `baseUrl: '/anvil/'`)
- New `apps/docs-public` Docusaurus build (Kindling, APS, edda-stack, blog)
- Shell middleware, auth routes, landing page, robots.txt, llms.txt
- Pulumi: three new Vercel projects, deployment protection on upstreams, domain cutover
- GitHub OAuth App callback URL update
- Retirement of `apps/docs-site`

## Out of Scope

- Migrating Anvil docs to MDX/Next.js directly
- Per-page ACLs inside `/anvil/*`
- Unified top-nav across public and private docs
- Better-Auth migration
- Refresh token rotation in the browser

## Interfaces

**Depends on:**

- DOCSAUTH (complete) — BAUTH GitHub OAuth endpoint, ES256 JWT, Key Vault secrets
- BAUTH (complete) — `/api/v1/auth/github/callback`
- IAC (complete) — Pulumi Vercel project pattern

**Exposes:**

- `docs.eddacraft.ai` as a Next.js shell origin
- Gated `/anvil/*` paths that actually prevent content delivery to unauthenticated clients
- `/auth/login`, `/auth/callback`, `/auth/logout`, `/auth/pending`, `/auth/error`
- `/robots.txt`, `/llms.txt`

## Constraints

- Docusaurus `baseUrl: '/anvil/'` must prefix every emitted asset including search index and sitemap (verified in Task 0)
- Shell middleware matcher must catch `/anvil/assets/*` chunks
- Cookie stays `HttpOnly; Secure; SameSite=Lax`
- Domain cutover is the only one-way door; all other steps are independently revertible

## Design Spec

`plans/specs/2026-04-11-docs-auth-spa-gap-design.md`

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (DOCSAUTH, BAUTH, IAC)
- [x] Design spec written and approved
- [x] Implementation plan written
- [ ] Kill-switch verification (Task 0) passed

---

## Phase 1 — docs-shell scaffolding (Tasks 1–15)

### DOCSAUTH2-001: Scaffold docs-shell project manifest

- **Status:** Ready
- **Intent:** Create `apps/docs-shell/` with Next.js 16 package.json, tsconfig, nx project.json, vitest config
- **Expected Outcome:** `pnpm nx typecheck docs-shell` succeeds on empty project
- **Plan reference:** Task 1
- **Dependencies:** Task 0

### DOCSAUTH2-002: lib/cookie.ts, lib/next-url.ts, lib/state.ts, lib/jwt.ts, lib/bauth.ts

- **Status:** Ready
- **Intent:** Write Edge-safe support libraries with full test coverage
- **Expected Outcome:** All five lib modules exist with passing vitest suites
- **Plan reference:** Tasks 2–6

### DOCSAUTH2-003: Middleware gate

- **Status:** Ready
- **Intent:** Implement JWT gate for `/anvil/:path*`
- **Plan reference:** Task 7

### DOCSAUTH2-004: Auth routes

- **Status:** Ready
- **Intent:** /auth/login, /auth/callback, /auth/logout, /auth/pending, /auth/error
- **Plan reference:** Tasks 8–11

### DOCSAUTH2-005: Landing page + robots + llms

- **Status:** Ready
- **Intent:** Root layout, landing page, /robots.txt, /llms.txt
- **Plan reference:** Tasks 12–13

### DOCSAUTH2-006: Shell config and preview deploy

- **Status:** Ready
- **Intent:** next.config.ts, vercel.json, Vercel preview deployment, smoke test
- **Plan reference:** Tasks 14–15

## Phase 2 — Docusaurus split (Tasks 16–19)

### DOCSAUTH2-007: Create anvil-docs-private build

- **Status:** Ready
- **Intent:** New Docusaurus project with Anvil plugin only and `baseUrl: '/anvil/'`
- **Expected Outcome:** Local build produces assets under `build/anvil/` only
- **Plan reference:** Task 16

### DOCSAUTH2-008: Deploy anvil-docs-private preview

- **Status:** Ready
- **Plan reference:** Task 17

### DOCSAUTH2-009: Create docs-public build from pruned docs-site

- **Status:** Ready
- **Plan reference:** Task 18

### DOCSAUTH2-010: Deploy docs-public preview

- **Status:** Ready
- **Plan reference:** Task 19

## Phase 3 — Integration (Task 20)

### DOCSAUTH2-011: Wire shell to real upstream URLs and full-flow smoke test

- **Status:** Ready
- **Plan reference:** Task 20

## Phase 4 — Infrastructure (Tasks 21–24)

### DOCSAUTH2-012: Pulumi — three new Vercel projects

- **Status:** Ready
- **Plan reference:** Task 21

### DOCSAUTH2-013: Pulumi — deployment protection on upstreams

- **Status:** Ready
- **Plan reference:** Task 22

### DOCSAUTH2-014: GitHub OAuth App callback URL update

- **Status:** Ready
- **Plan reference:** Task 23

### DOCSAUTH2-015: Pulumi — domain cutover

- **Status:** Ready
- **Priority:** High (one-way door)
- **Plan reference:** Task 24

## Phase 5 — Cutover & retirement (Tasks 25–26)

### DOCSAUTH2-016: Production smoke test

- **Status:** Ready
- **Plan reference:** Task 25

### DOCSAUTH2-017: Retire apps/docs-site

- **Status:** Ready
- **Plan reference:** Task 26
- **Dependencies:** DOCSAUTH2-016 green for 24h

## Risks

Inherited from `plans/specs/2026-04-11-docs-auth-spa-gap-design.md` §Risks. Key ones:

| Risk | Mitigation |
| ---- | ---------- |
| `baseUrl` asset leak | Task 0 kill-switch; fallback to subdomain split |
| Deployment Protection not on plan | Shared-secret header fallback in Task 22 |
| One-way domain cutover | 24h observation gate before Task 26 |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Shell scaffold | 6 | 0/6 |
| 2 — Docusaurus split | 4 | 0/4 |
| 3 — Integration | 1 | 0/1 |
| 4 — Infrastructure | 4 | 0/4 |
| 5 — Cutover & retirement | 2 | 0/2 |
| **Total** | **17** | **0/17** |
```

- [ ] **Step 3: Commit**

```bash
git add plans/modules/docs-auth-spa-gap.aps.md
git commit -m "plan(docsauth2): add APS module for SPA gap remediation"
```

---

## Task 28 — Update CLAUDE.md module index

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Add DOCSAUTH2 to the active modules list**

Edit `/home/aneki/Projects/src/EddaCraft/anvil-001/CLAUDE.md`. Find the `## Active Modules` section. Update DOCSAUTH status to `7/7 Complete` (it was stale at 6/7). Add DOCSAUTH2 as `In Progress`.

```diff
- DOCSAUTH: docs-auth-gating (6/7) — In Progress
+ DOCSAUTH: docs-auth-gating (7/7) — Complete
+ DOCSAUTH2: docs-auth-spa-gap (0/17) — In Progress
```

- [ ] **Step 2: Add new file map entries**

Find the File Map section and append:

```
apps/docs-shell/middleware.ts: DOCSAUTH2-003
apps/docs-shell/lib/: DOCSAUTH2-002
apps/docs-shell/app/auth/: DOCSAUTH2-004
apps/docs-shell/app/page.tsx: DOCSAUTH2-005
apps/docs-shell/app/robots.txt/route.ts: DOCSAUTH2-005
apps/docs-shell/app/llms.txt/route.ts: DOCSAUTH2-005
apps/docs-shell/next.config.ts: DOCSAUTH2-006
apps/anvil-docs-private/docusaurus.config.ts: DOCSAUTH2-007
apps/docs-public/docusaurus.config.ts: DOCSAUTH2-009
infra/src/vercel.ts: DOCSAUTH2-012, DOCSAUTH2-013, DOCSAUTH2-015
```

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude): register DOCSAUTH2 module and mark DOCSAUTH complete"
```

---

## Self-Review

**Spec coverage check:**

- ✅ Context / SPA bypass / threat model — no code needed, covered by task ordering and Task 0 kill-switch
- ✅ Goals: all five bullets covered
  - No Anvil bytes to unauth clients → Tasks 7 (middleware), 16 (baseUrl), 22 (deployment protection)
  - Cross-links feel native → Task 12 (landing) + same-origin rewrites (Task 14)
  - Public docs remain public → Task 18
  - BAUTH reuse → Task 6 (bauth.ts) + Task 9 (callback)
  - Better-Auth-survivable → Task 8–10 auth routes use stable cookie interface
  - 2-3 day deployable → 29 tasks (Task 0 scaffold + Tasks 1–28) at 2-5 min per step, ~140 steps, ~8–12 hours of work
- ✅ Non-goals: all honored (no MDX migration, no per-page ACLs, no Clerk, no Better-Auth)
- ✅ Architecture diagram: Tasks 1, 14, 16, 18 build it piece by piece
- ✅ `baseUrl` kill-switch: Task 0 explicitly verifies
- ✅ Middleware: Task 7
- ✅ Auth routes: Tasks 8–11
- ✅ Rewrites: Task 14
- ✅ robots.txt/llms.txt: Task 13
- ✅ Pulumi changes: Tasks 21, 22, 24
- ✅ Migration plan: Tasks 0, 1–15, 16–19, 20, 21–24, 25, 26 — same 9-step order as spec §Migration plan
- ✅ Policy posture: not a code concern, lives in BAUTH runtime config, not reopened here
- ✅ Rollback: Task 24 is flagged as the one-way door; Task 26 has 24h observation gate

**Placeholder scan:** No "TBD", "TODO", "implement later" in task steps. All code blocks contain complete, runnable content. Imports are explicit. Env vars are named concretely.

**Type consistency:**
- `verifyLicense()` signature in Task 5 matches the call in Task 7 middleware
- `exchangeGithubCode()` return type in Task 6 matches the destructuring in Task 9 callback
- `encryptState(payload, secret)` / `decryptState(encrypted, secret)` parameter order matches across Tasks 4, 8, 9
- `validateNext()` signature matches Task 3 definition in both Task 8 and Task 9 callers
- Cookie names consistent: `anvil-docs-session` everywhere, `oauth-nonce` in login/callback
- `COOKIE_MAX_AGE = 7 * 24 * 60 * 60` matches spec's "7 days" cookie attribute

**Gap found and fixed inline:** Task 14 referenced `ANVIL_DOCS_URL` / `PUBLIC_DOCS_URL` env vars but Task 15 (preview deploy) did not set them. Task 20 was added to cover the env var wiring after upstream previews exist.

**Gap found and fixed inline:** The spec mentions a `sitemap.xml` route on the shell but the plan does not create one. **Decision:** defer to post-launch — sitemap belongs to the public build (`docs-public/build/sitemap.xml`), which the shell's public rewrites already proxy. Shell does not need to emit its own.

**Gap found and fixed inline:** Spec mentions `X-Robots-Tag` header on `/anvil/*` responses. Added to `next.config.ts` headers section in Task 14.

---

## Execution Handoff

Plan complete and saved to `plans/specs/2026-04-11-docs-auth-spa-gap-plan.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Good for a 28-task plan with distinct boundaries between phases.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints. Faster end-to-end but you see less review granularity.

Which approach?
