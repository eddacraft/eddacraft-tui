import { SignJWT, importPKCS8, importSPKI, jwtVerify, type CryptoKey } from 'jose';

const LICENCE_TTL_DAYS = 90;
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
  tier: string;
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
    return {
      sub: payload.sub,
      email: payload['email'] as string,
      identity: identity ?? { provider: 'unknown', id: null },
      org: (payload['org'] as string | null) ?? null,
      tier: typeof payload['tier'] === 'string' ? (payload['tier'] as string) : 'pro',
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
    tier: claims.tier,
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
