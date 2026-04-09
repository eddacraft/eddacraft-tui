import { SignJWT, importPKCS8 } from 'jose';

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

export async function signLicence(
  claims: LicenceClaims,
  tokenExpiresAt?: string | Date,
  ttlDays?: number
): Promise<string> {
  const pem = process.env['LICENSE_SIGNING_KEY'];
  if (!pem) {
    throw new Error('LICENSE_SIGNING_KEY environment variable is required');
  }

  const privateKey = await importPKCS8(pem, 'ES256');
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
