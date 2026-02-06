# Adversarial Code Review: apps/anvil-api

**Date:** 2026-02-06
**Reviewer:** Claude (automated adversarial review)
**Scope:** Full `apps/anvil-api` codebase (~466 production LOC, 8 source files)

---

## Executive Summary

The anvil-api is a compact, well-architected beta access API built on Hono + Neon
Postgres, deployed to Vercel. At only 466 production lines, the attack surface is small
and the code quality is high. The most notable positives: tokens are stored as SHA-256
hashes with optional pepper, admin auth uses timing-safe comparison, all input is
Zod-validated, and all SQL is parameterised (no injection surface). The review identified
**2 high-severity concerns** (wide-open CORS, no rate limiting), **5 medium-severity
issues** (missing output validation, accountability gaps), and **5 low-severity items**.

### Severity Counts

| Severity | Count |
|----------|-------|
| CRITICAL | 0 |
| HIGH     | 2 |
| MEDIUM   | 5 |
| LOW      | 5 |

---

## CRITICAL Issues

None.

---

## HIGH Issues

### H1. CORS allows all origins — admin endpoints callable from any website

**File:** `src/index.ts:10`

```typescript
app.use('*', cors());
```

Hono's `cors()` with no arguments sets `Access-Control-Allow-Origin: *` and allows
all methods and headers. This means:

1. Any website can make cross-origin `POST` requests to `/api/v1/admin/invite`,
   `/admin/revoke`, etc., provided the `Authorization` header is included.
2. A malicious page visited by an admin whose browser has the admin key (e.g., via a
   browser extension, local proxy, or saved `fetch` call in devtools) could
   silently create users and generate tokens.
3. The `/api/v1/auth/verify` endpoint is publicly accessible by design, but the
   admin endpoints should not be.

While bearer token auth is present, CORS is the browser's first line of defence
against cross-site admin API abuse.

**Recommendation:** Restrict CORS to known origins:
```typescript
app.use('*', cors({ origin: ['https://admin.anvil.dev'], credentials: true }));
```
Or apply restrictive CORS only to `/admin/*` routes while keeping `/auth/*` open.

---

### H2. No rate limiting on any endpoint

**File:** `src/routes/auth.ts:20`, `src/routes/admin.ts`

The API has no rate limiting at any layer:

1. **`POST /auth/verify`** — An attacker can test tokens at line speed. While the
   token space is 256 bits (infeasible to brute-force), a leaked partial token list
   from logs or a compromised client could be tested in bulk. More practically, a
   flood of requests can overwhelm the Neon serverless database.
2. **`POST /admin/invite`** — Without rate limiting, a compromised admin key allows
   mass user/token creation at database write speed.
3. **`POST /admin/revoke`** — Bulk revocation attacks could disrupt all beta users.

Vercel may provide some DDoS protection, but application-level rate limiting is needed
for abuse prevention and cost control (Neon bills per query).

**Recommendation:** Add `hono-rate-limiter` or a lightweight middleware that tracks
requests per IP/key. At minimum:
- `/auth/verify`: 60 req/min per IP
- `/admin/*`: 30 req/min per admin key

---

## MEDIUM Issues

### M1. Audit log actor is always `'admin'` — no individual accountability

**File:** `src/routes/admin.ts:58, 86, 96`

```typescript
await insertAuditLog(sql, 'token.created', 'admin', { email, scopes, days });
await insertAuditLog(sql, 'tokens.revoked', 'admin', { email, count });
await insertAuditLog(sql, 'token.revoked', 'admin', { revoked });
```

All audit entries record the actor as the literal string `'admin'`. With a single
shared `ADMIN_KEY`, there is no way to distinguish which person performed an action.
If multiple team members have the admin key, the audit log cannot answer "who invited
this user?" or "who revoked these tokens?"

**Recommendation:** Either:
1. Support per-admin API keys with a `admin_users` table, or
2. Accept an `X-Admin-Actor` header (validated but not auth'd) to at least record
   who claims to be performing the action, or
3. Log the source IP and User-Agent alongside the action for forensics.

---

### M2. Database query results are cast without runtime validation

**File:** `src/db/queries.ts:32-34, 42, 61, 78, 94, etc.`

```typescript
function rows(result: unknown): Record<string, unknown>[] {
  return result as Record<string, unknown>[];
}

// Then every query:
return (r[0] as BetaUser) ?? null;
return r[0] as AccessToken;
```

All query results are double-cast: first through `rows()` to `Record<string, unknown>[]`,
then to the expected interface. No runtime validation occurs. If:
1. The database schema drifts from the TypeScript interfaces (e.g., a column rename
   or type change),
2. A query returns unexpected data (e.g., from a modified view),
3. The Neon client changes its response format in a major version bump,

the API will silently pass malformed data to callers, potentially exposing internal
DB structure or causing hard-to-debug runtime errors.

This is inconsistent with the excellent input validation (Zod on all request bodies).

**Recommendation:** Define Zod schemas for each query result type and validate in
`rows()` or at each call site. Example:
```typescript
const BetaUserSchema = z.object({ id: z.string(), email: z.string(), ... });
```

---

### M3. `GET /admin/user/:email` — URL parameter has no format validation

**File:** `src/routes/admin.ts:110-111`

```typescript
admin.get('/user/:email', async (c) => {
  const email = c.req.param('email').toLowerCase().trim();
  const sql = getClient();
  const result = await findUserWithTokens(sql, email);
```

The POST routes use Zod schemas with `z.string().email()` validation, but this GET
route passes the raw URL parameter directly to the database query after only
`toLowerCase().trim()`. While the parameterised query prevents SQL injection, an
invalid email like `../../../etc/passwd` or a very long string would still hit the
database unnecessarily.

**Recommendation:** Validate the email parameter before querying:
```typescript
const emailSchema = z.string().email();
const result = emailSchema.safeParse(email);
if (!result.success) return c.json({ error: 'Invalid email format' }, 400);
```

---

### M4. `scopes` field accepts arbitrary strings with no allowlist

**File:** `src/routes/admin.ts:21`

```typescript
scopes: z.array(z.string()).default(['beta']),
```

Any string array is accepted as valid scopes: `["admin", "superuser", ""]`,
`["delete-all"]`, etc. If the consuming CLI or other services check scopes to gate
functionality, an admin could create tokens with unintended privilege levels.

**Recommendation:** Validate scopes against a known allowlist:
```typescript
const VALID_SCOPES = ['beta', 'preview', 'internal'] as const;
scopes: z.array(z.enum(VALID_SCOPES)).default(['beta']),
```

---

### M5. Audit log failure crashes the request after main operation completes

**File:** `src/routes/admin.ts:58-62`

```typescript
const rawToken = generateToken();
const hash = hashToken(rawToken);
// ... token inserted into DB successfully ...

await insertAuditLog(sql, 'token.created', 'admin', {   // ← if this fails...
  email: normalizedEmail,
  scopes,
  days,
});

return c.json({ token: rawToken, ... }, 201);  // ← ...this never runs
```

The invite flow is: create user → generate token → insert token → write audit log →
return token. If the audit log insert fails (DB connection drop, schema issue,
constraint violation), the HTTP response is a 500 error, but the token has already
been created in the database.

The admin sees an error, retries, and creates a duplicate token. The original
(invisible) token is valid but the admin never saw it.

Same pattern in revoke routes (lines 86-89, 96-98).

**Recommendation:** Either:
1. Wrap the entire operation in a database transaction (so audit failure rolls back
   the token creation), or
2. Catch audit log errors and still return the successful response with a warning:
   ```typescript
   try { await insertAuditLog(...); } catch (e) { /* log error but don't fail request */ }
   ```

---

## LOW Issues

### L1. Health endpoint leaks server timestamp with millisecond precision

**File:** `src/index.ts:13`

```typescript
return c.json({ status: 'ok', timestamp: new Date().toISOString() });
```

Reveals the server's wall clock time, which can be useful for:
1. Timing attack calibration (knowing the server's time offset)
2. Correlating requests across distributed systems for fingerprinting

Low severity because this is standard practice for health endpoints and the
information gain is minimal.

---

### L2. No request body size limits

The Hono app does not configure body size limits. A malicious client could send:
1. A multi-MB `notes` field in `/admin/invite`
2. A multi-MB `token` string in `/auth/verify`

Zod validates shape but not size. The Neon client would attempt to insert the
oversized data into the database.

**Recommendation:** Add body size middleware or Zod `.max()` constraints:
```typescript
notes: z.string().max(1000).optional(),
token: z.string().max(100),
```

---

### L3. TOKEN_PEPPER rotation silently invalidates all existing tokens

**File:** `src/lib/token.ts:22`

```typescript
const pepper = process.env['TOKEN_PEPPER'] ?? '';
```

If `TOKEN_PEPPER` is changed, rotated, or accidentally removed, all existing token
hashes become invalid because `sha256(newPepper + rawToken) !== sha256(oldPepper + rawToken)`.
Users would silently lose access with `{valid: false}` and no error explaining why.

There is no mechanism to:
1. Migrate existing hashes after a pepper change
2. Support multiple peppers during rotation
3. Detect or alert when a pepper mismatch is likely

---

### L4. `setClient()` is exported in production code

**File:** `src/db/client.ts:19-21`

```typescript
export function setClient(client: NeonClient): void {
  _client = client;
}
```

While intended for testing, this function is exported from the production module.
Any code that can import `@eddacraft/anvil-api/db/client` can redirect all database
queries to an arbitrary backend. In practice this requires code execution privilege
(which is game over anyway), but exporting test-only functions in production is poor
hygiene.

**Recommendation:** Gate behind `NODE_ENV` check or move to a test-only module.

---

### L5. No database connection validation at startup

**File:** `src/db/client.ts:7-16`

```typescript
export function getClient(): NeonClient {
  if (!_client) {
    const url = process.env['DATABASE_URL'];
    if (!url) throw new Error('DATABASE_URL environment variable is required');
    _client = neon(url);
  }
  return _client;
}
```

The Neon client is lazily created on first request. If `DATABASE_URL` is syntactically
valid but the database is unreachable (wrong password, network issue, DB deleted), the
first user request will fail with an opaque error. The `/health` endpoint returns
`{status: 'ok'}` even when the database is down.

**Recommendation:** Add a DB ping to the health endpoint:
```typescript
app.get('/health', async (c) => {
  try {
    const sql = getClient();
    await sql`SELECT 1`;
    return c.json({ status: 'ok' });
  } catch {
    return c.json({ status: 'degraded', db: 'unreachable' }, 503);
  }
});
```

---

## Architectural Observations

### Positive Patterns

1. **Token hashing with optional pepper** — raw tokens are never stored; SHA-256
   hashes with `TOKEN_PEPPER` provide defence-in-depth against DB compromise.
2. **Timing-safe admin auth** — `admin-auth.ts` uses `timingSafeEqual` correctly,
   including the length check before comparison.
3. **Zod validation on all POST bodies** — `@hono/zod-validator` enforces schemas
   before handlers run. The `refine()` on `revokeSchema` is well-crafted.
4. **Parameterised SQL** — all queries use Neon's tagged template literals, which
   auto-parameterise. Zero SQL injection surface.
5. **No error detail leakage** — `auth/verify` always returns 200 with `{valid: false}`
   regardless of failure reason. Admin auth returns generic 401/403.
6. **Token hash not exposed in GET response** — `admin.ts:121-127` explicitly maps
   token fields, excluding `token_hash`. Test verifies this (line 197).
7. **Email normalisation** — `toLowerCase().trim()` applied consistently before DB
   operations, combined with `citext` column type.
8. **Minimal dependency surface** — only 4 production dependencies (hono, zod,
   zod-validator, neon). Small supply chain risk.
9. **Append-only audit log** — `audit_log` table has no UPDATE/DELETE operations in
   the query layer, preserving integrity.

### Negative Patterns

1. **Input validation without output validation** — Zod is used rigorously on
   request bodies but never on database query results. Trust boundary is only at
   the edge, not at the DB layer.
2. **No transactions** — multi-step operations (insert token + audit log) are not
   wrapped in transactions, allowing partial completion.
3. **Single shared admin key** — no per-admin identity, making the audit log
   less useful for accountability.

---

## Recommendations Priority

| Priority | Action |
|----------|--------|
| P0 | Restrict CORS to known admin origins (H1) |
| P0 | Add rate limiting to all endpoints (H2) |
| P1 | Validate scopes against allowlist (M4) |
| P1 | Wrap invite/revoke in DB transactions or handle audit failure (M5) |
| P1 | Add Zod validation to DB query results (M2) |
| P2 | Add email validation to GET /admin/user/:email (M3) |
| P2 | Track admin identity in audit logs (M1) |
| P2 | Add body size limits (L2) |
| P3 | Add DB health check to /health endpoint (L5) |
| P3 | Document TOKEN_PEPPER rotation procedure (L3) |
| P3 | Gate setClient() behind NODE_ENV (L4) |
