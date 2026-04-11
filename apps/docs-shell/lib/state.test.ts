// apps/docs-shell/lib/state.test.ts
import { describe, it, expect } from 'vitest';
import { encryptState, decryptState } from './state';

const SECRET = 'test-secret-at-least-32-bytes-long-for-aes';

describe('state encryption', () => {
  it('roundtrips a payload', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const encrypted = await encryptState(payload, SECRET);
    const decrypted = await decryptState(encrypted, SECRET);
    expect(decrypted).toEqual(payload);
  });

  it('produces different ciphertext for the same input (random IV)', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const a = await encryptState(payload, SECRET);
    const b = await encryptState(payload, SECRET);
    expect(a).not.toBe(b);
  });

  it('returns null when decrypting with the wrong secret', async () => {
    const payload = { next: '/anvil/overview', nonce: 'abc123' };
    const encrypted = await encryptState(payload, SECRET);
    const decrypted = await decryptState(encrypted, 'different-secret-also-long-enough');
    expect(decrypted).toBeNull();
  });

  it('returns null for garbled input', async () => {
    expect(await decryptState('not-valid-base64url', SECRET)).toBeNull();
  });

  it('returns null for truncated input', async () => {
    expect(await decryptState('AA', SECRET)).toBeNull();
  });
});
