import { SignJWT, importPKCS8, type CryptoKey } from 'jose';

const LICENCE_TTL_DAYS = 90;
const RC_AFTER_DAYS = 7;
const DEFAULT_KEY_ID = '2026-03';
const KEY_ID = process.env['LICENSE_PUBLIC_KEY_KID'] ?? DEFAULT_KEY_ID;

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

function loadSigningKey(): Promise<CryptoKey> {
  if (cachedSigningKey) return cachedSigningKey;
  const pem = process.env['LICENSE_SIGNING_KEY'];
  if (!pem) {
    return Promise.reject(new Error('LICENSE_SIGNING_KEY environment variable is required'));
  }
  cachedSigningKey = importPKCS8(pem, 'ES256');
  return cachedSigningKey;
}

export async function verifySigningKey(): Promise<{ ok: true } | { ok: false; error: string }> {
  try {
    await loadSigningKey();
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}

// Test-only: clears the module-level cache so env-var changes take effect.
// Production callers mutate env vars once at boot — a runtime rotation path
// would need a proper invalidation strategy, not this.
export function _resetSigningKeyCacheForTests(): void {
  cachedSigningKey = null;
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
