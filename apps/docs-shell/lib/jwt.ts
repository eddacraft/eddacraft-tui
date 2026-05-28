import { jwtVerify, importSPKI, type CryptoKey } from 'jose';

let cachedKey: CryptoKey | null = null;
const DOCS_ACCESS_TIERS = new Set(['beta', 'pro', 'enterprise']);

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
    const { payload } = await jwtVerify(token, publicKey, { algorithms: ['ES256'] });
    if (typeof payload['tier'] !== 'string' || !DOCS_ACCESS_TIERS.has(payload['tier'])) {
      return { valid: false };
    }
    return { valid: true };
  } catch {
    return { valid: false };
  }
}
