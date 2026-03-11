import { jwtVerify, importSPKI, decodeProtectedHeader, errors, type JWTPayload } from 'jose';
import { debug } from '../utils/output.js';
import { LICENCE_PUBLIC_KEYS } from './licence-keys.js';

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

let publicKeysPem: Record<string, string> = {};

export function setPublicKeys(keys: Record<string, string>): void {
  publicKeysPem = keys;
}

export async function verifyLicence(jwt: string): Promise<LicenceResult> {
  try {
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
        scopes: (payload as Record<string, unknown>).scopes as string[] | undefined,
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

// Load baked-in keys on import (tests can override via setPublicKeys)
if (Object.keys(publicKeysPem).length === 0 && Object.keys(LICENCE_PUBLIC_KEYS).length > 0) {
  setPublicKeys(LICENCE_PUBLIC_KEYS);
}
