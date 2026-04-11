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
  const padded =
    str.replaceAll('-', '+').replaceAll('_', '/') + '='.repeat((4 - (str.length % 4)) % 4);
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

export async function decryptState(
  encrypted: string,
  secret: string
): Promise<StatePayload | null> {
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
