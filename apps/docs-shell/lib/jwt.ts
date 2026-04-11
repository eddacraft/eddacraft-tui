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
