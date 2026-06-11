import { describe, it, expect } from 'vitest';
import { encryptDeviceCode, decryptDeviceCode } from '../lib/github-device-crypto.js';

const POLL_TOKEN = 'a'.repeat(64);
const DEVICE_CODE = '3584d83530557fdd1f46af8289938c8ef79f9dc5';

describe('github-device-crypto', () => {
  it('round-trips a device code under the issuing poll token', () => {
    const payload = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    expect(decryptDeviceCode(POLL_TOKEN, payload)).toBe(DEVICE_CODE);
  });

  it('never stores the device code recoverable without the poll token', () => {
    const payload = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    expect(payload).not.toContain(DEVICE_CODE);
    expect(Buffer.from(payload).toString('hex')).not.toContain(
      Buffer.from(DEVICE_CODE).toString('hex')
    );
  });

  it('fails closed under a different poll token', () => {
    const payload = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    expect(decryptDeviceCode('b'.repeat(64), payload)).toBeNull();
  });

  it('fails closed on a tampered payload', () => {
    const payload = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    const parts = payload.split('.');
    // Tamper at the byte level, not the base64url text level: the final
    // encoded character of a partial group carries ignored trailing bits, so
    // a character swap there can decode to identical bytes and slip past GCM.
    const ciphertext = Buffer.from(parts[3]!, 'base64url');
    ciphertext[0] = ciphertext[0]! ^ 0xff;
    parts[3] = ciphertext.toString('base64url');
    expect(decryptDeviceCode(POLL_TOKEN, parts.join('.'))).toBeNull();
  });

  it('fails closed on a malformed payload', () => {
    expect(decryptDeviceCode(POLL_TOKEN, 'not-a-payload')).toBeNull();
    expect(decryptDeviceCode(POLL_TOKEN, '')).toBeNull();
    expect(decryptDeviceCode(POLL_TOKEN, 'v2.a.b.c')).toBeNull();
  });

  it('produces a fresh ciphertext per call (random IV)', () => {
    const a = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    const b = encryptDeviceCode(POLL_TOKEN, DEVICE_CODE);
    expect(a).not.toBe(b);
    expect(decryptDeviceCode(POLL_TOKEN, a)).toBe(DEVICE_CODE);
    expect(decryptDeviceCode(POLL_TOKEN, b)).toBe(DEVICE_CODE);
  });
});
