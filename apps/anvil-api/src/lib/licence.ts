import { SignJWT, importPKCS8, importSPKI, jwtVerify, type CryptoKey } from 'jose';

const LICENCE_TTL_DAYS = 90;
/**
 * SEC-012: the exact legacy `tier` value we ever minted, and the plan it
 * de-escalates to. Pre-BACT-013 `signLicence` hardcoded `tier: 'pro'` for
 * every licence (`lib/session.ts:85,165` at `f94ba7fe5^`), so `'pro'` is the
 * only legacy shape that exists on the wire. Matching it exactly — rather
 * than de-escalating any string — means a `tier` value we never issued is
 * denied instead of being handed `beta`, which would be an *elevation* for
 * e.g. a `tier: 'free'` token. `beta` is what those accounts actually hold
 * (`beta_users.plan` CHECK). Delete both constants and the branch in
 * `verifyLicence` once the last pre-BACT-013 licence expires (~2026-11-11).
 */
const LEGACY_TIER_VALUE = 'pro';
const STALE_TIER_PLAN = 'beta';
const RC_AFTER_DAYS = 7;
const DEFAULT_KEY_ID = '2026-03';
const KEY_ID = process.env['LICENSE_PUBLIC_KEY_KID'] ?? DEFAULT_KEY_ID;
const LICENCE_ISSUER = 'https://api.eddacraft.ai';
const LICENCE_AUDIENCE = 'anvil-cli';

export interface LicenceClaims {
  sub: string;
  email: string;
  identity: { provider: string; id: string | null };
  org: string | null;
  /**
   * BACT-013 / ADR-121 decision 6: durable plan name sourced from
   * `beta_users.plan` (e.g. `beta`). The entitlement axis — supersedes the
   * legacy `tier` name, which described a subscription tier ("pro") that
   * never actually varied and confused the plan axis. `signLicence` still
   * writes a `tier` key onto the wire (mirroring this value) for
   * `apps/docs-shell` and `apps/docs-site`, which verify the raw JWT without
   * a DB round trip (OQ-C).
   *
   * **SEC-012:** `null` on the verify side means *no entitlement claim was
   * present*. Gates must treat null as unentitled — it is never coerced to a
   * plan name. Minting requires a real plan; `signLicence` rejects null.
   */
  plan: string | null;
  scopes: string[];
  seats: number;
}

let cachedSigningKey: Promise<CryptoKey> | null = null;
let cachedVerifyingKey: Promise<CryptoKey> | null = null;

function loadSigningKey(): Promise<CryptoKey> {
  if (cachedSigningKey) return cachedSigningKey;
  const pem = process.env['LICENSE_SIGNING_KEY'];
  if (!pem) {
    return Promise.reject(new Error('LICENSE_SIGNING_KEY environment variable is required'));
  }
  cachedSigningKey = importPKCS8(pem, 'ES256');
  return cachedSigningKey;
}

function loadVerifyingKey(): Promise<CryptoKey> {
  if (cachedVerifyingKey) return cachedVerifyingKey;
  const pem = process.env['LICENSE_PUBLIC_KEY'];
  if (!pem) {
    return Promise.reject(new Error('LICENSE_PUBLIC_KEY environment variable is required'));
  }
  cachedVerifyingKey = importSPKI(pem, 'ES256');
  return cachedVerifyingKey;
}

export async function verifySigningKey(): Promise<{ ok: true } | { ok: false; error: string }> {
  try {
    await loadSigningKey();
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

/**
 * Probe `LICENSE_PUBLIC_KEY` so a missing or invalid SPKI is caught at boot
 * / `/health` rather than at the first authenticated request. Mirrors
 * [`verifySigningKey`] for the verification half of the keypair.
 */
export async function verifyVerifyingKey(): Promise<{ ok: true } | { ok: false; error: string }> {
  try {
    await loadVerifyingKey();
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

// Test-only: clears the module-level caches so env-var changes take effect.
// Production callers mutate env vars once at boot — a runtime rotation path
// would need a proper invalidation strategy, not this.
export function _resetSigningKeyCacheForTests(): void {
  cachedSigningKey = null;
  cachedVerifyingKey = null;
}

/**
 * Verify an anvil-issued licence JWT and return the parsed claims if the
 * signature, issuer, audience, expiry (`exp`), and not-before (`nbf`, when
 * present) are all valid per `jwtVerify`'s defaults. Returns `null` on any
 * verification failure — callers MUST treat a null return as "no
 * authenticated identity" and refuse the request. Never silently downgrade.
 *
 * `iat` is **not** enforced — `jwtVerify` does not validate issued-at by
 * default and we do not pass a `maxTokenAge` policy. Token expiry is
 * controlled by `exp` instead (set by [`signLicence`]). If a maximum token
 * age is needed in the future, add a `maxTokenAge` option here.
 *
 * The verification key (`LICENSE_PUBLIC_KEY`) is required. A missing or
 * invalid key surfaces as a **thrown error** rather than a silent allow —
 * callers MUST distinguish that configuration failure from a verification
 * failure (e.g. respond 500 vs 401).
 */
export async function verifyLicence(jwt: string): Promise<LicenceClaims | null> {
  const key = await loadVerifyingKey();
  try {
    const { payload } = await jwtVerify(jwt, key, {
      issuer: LICENCE_ISSUER,
      audience: LICENCE_AUDIENCE,
      algorithms: ['ES256'],
    });
    if (
      typeof payload.sub !== 'string' ||
      typeof payload['email'] !== 'string' ||
      !Array.isArray(payload['scopes'])
    ) {
      return null;
    }
    const identity = payload['identity'] as LicenceClaims['identity'] | undefined;
    // SEC-012: resolve the entitlement claim without ever letting a token
    // elevate itself. Three cases, in order:
    //
    //   1. `plan` present         -> use it verbatim (post-BACT-013 shape).
    //   2. `tier` === 'pro' only  -> STALE_TIER_PLAN, *not* the tier's value.
    //   3. anything else          -> null; the gate denies.
    //
    // Case 2 replaces the BACT-013 rule that promoted `tier` into `plan` to
    // avoid "silently downgrading an in-flight session". That optimised for
    // continuity in a world where every plan grants the same thing. It is an
    // over-entitlement vector the moment plans differentiate: pre-BACT-013
    // licences carry `tier: 'pro'`, no account has ever held `pro`
    // (`beta_users.plan` CHECK admits only `beta`), and `apps/docs-shell`
    // and `apps/docs-site` read this claim with no DB round trip — so they
    // would believe it. De-escalating keeps those sessions working at the
    // plan they actually hold, with no forced re-authentication, and the
    // alias retires by natural expiry once the last pre-BACT-013 licence
    // ages out (90-day TTL from 2026-08-13 => ~2026-11-11). At that point
    // case 2 can be deleted outright.
    const rawPlan = payload['plan'];
    const rawTier = payload['tier'];
    const plan =
      typeof rawPlan === 'string'
        ? rawPlan
        : rawTier === LEGACY_TIER_VALUE
          ? STALE_TIER_PLAN
          : null;
    return {
      sub: payload.sub,
      email: payload['email'] as string,
      identity: identity ?? { provider: 'unknown', id: null },
      org: (payload['org'] as string | null) ?? null,
      plan,
      scopes: payload['scopes'] as string[],
      seats: typeof payload['seats'] === 'number' ? (payload['seats'] as number) : 1,
    };
  } catch {
    return null;
  }
}

export async function signLicence(
  claims: LicenceClaims,
  tokenExpiresAt?: string | Date,
  ttlDays?: number
): Promise<string> {
  // SEC-012: `plan` is nullable on the verify side (a claimless legacy
  // token), never on the mint side. Callers build claims from
  // `beta_users.plan`, a NOT NULL column, so a null here is a caller bug —
  // fail loudly rather than minting a licence no gate can evaluate.
  if (claims.plan === null) {
    throw new Error('signLicence: claims.plan is required — cannot mint a licence with no plan');
  }
  const privateKey = await loadSigningKey();
  const now = Math.floor(Date.now() / 1000);
  const effectiveTtl = ttlDays ?? LICENCE_TTL_DAYS;
  const maxExp = now + effectiveTtl * 86400;
  let tokenExp = maxExp;
  if (tokenExpiresAt) {
    const parsed = Math.floor(new Date(tokenExpiresAt).getTime() / 1000);
    if (Number.isNaN(parsed)) {
      throw new Error(`Invalid tokenExpiresAt: ${String(tokenExpiresAt)}`);
    }
    tokenExp = parsed;
  }
  const exp = Math.min(tokenExp, maxExp);

  return new SignJWT({
    email: claims.email,
    identity: claims.identity,
    org: claims.org,
    plan: claims.plan,
    // BACT-013 / OQ-C: `tier` compat alias for apps/docs-shell and
    // apps/docs-site, which verify the raw JWT at the edge and still read
    // `tier` directly (DOCS_ACCESS_TIERS / evaluateDocsAccess). Mirrors
    // `plan` byte-for-byte — never a second semantic axis (ADR-121 decision
    // 6). Drop once those edge verifiers read `plan` instead.
    tier: claims.plan,
    scopes: claims.scopes,
    seats: claims.seats,
    rcAfter: now + RC_AFTER_DAYS * 86400,
  })
    .setProtectedHeader({ alg: 'ES256', kid: KEY_ID })
    .setSubject(claims.sub)
    .setIssuedAt(now)
    .setIssuer('https://api.eddacraft.ai')
    .setAudience('anvil-cli')
    .setExpirationTime(exp)
    .sign(privateKey);
}
