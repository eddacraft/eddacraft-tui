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
  const tokenExp = tokenExpiresAt ? Math.floor(new Date(tokenExpiresAt).getTime() / 1000) : maxExp;
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
    .setExpirationTime(exp)
    .sign(privateKey);
}
