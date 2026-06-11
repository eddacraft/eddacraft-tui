import { createCipheriv, createDecipheriv, hkdfSync, randomBytes } from 'node:crypto';

/**
 * At-rest protection for the GitHub `device_code` held by a
 * `github_device_sessions` row (GHCLIAUTH-004, ADR-066).
 *
 * The poll broker must send the *plaintext* device_code to GitHub's token
 * endpoint on every poll (RFC 8628 §3.4), so a one-way hash cannot work here.
 * Instead the code is encrypted under a key derived from the client-held
 * `poll_token` — which the DB only stores as a hash. The property the ADR's
 * "hashed at rest" invariant is after still holds: a DB dump alone recovers
 * nothing; only the CLI presenting the original poll_token lets the broker
 * decrypt the device_code for the exchange.
 */

const VERSION = 'v1';
const HKDF_INFO = 'github-device-code-v1';
const KEY_BYTES = 32;
const IV_BYTES = 12;

function deriveKey(pollToken: string): Buffer {
  // TOKEN_PEPPER as HKDF salt: matches the hashToken() trust model — an
  // attacker with the DB but not the env still cannot brute-force short tokens.
  const pepper = process.env['TOKEN_PEPPER'] ?? '';
  return Buffer.from(hkdfSync('sha256', pollToken, pepper, HKDF_INFO, KEY_BYTES));
}

/**
 * Encrypt a GitHub device_code under the session's poll_token.
 * Payload format: `v1.<iv>.<tag>.<ciphertext>` (base64url fields).
 */
export function encryptDeviceCode(pollToken: string, deviceCode: string): string {
  const iv = randomBytes(IV_BYTES);
  const cipher = createCipheriv('aes-256-gcm', deriveKey(pollToken), iv);
  const ciphertext = Buffer.concat([cipher.update(deviceCode, 'utf8'), cipher.final()]);
  const tag = cipher.getAuthTag();
  return [
    VERSION,
    iv.toString('base64url'),
    tag.toString('base64url'),
    ciphertext.toString('base64url'),
  ].join('.');
}

/**
 * Decrypt a stored device_code payload with the poll_token presented by the
 * CLI. Returns null on any mismatch — wrong token, tampered payload, or an
 * unknown format — so callers fail closed into the "expired" path.
 */
export function decryptDeviceCode(pollToken: string, payload: string): string | null {
  try {
    const parts = payload.split('.');
    if (parts.length !== 4) return null;
    const [version, ivPart, tagPart, ciphertextPart] = parts;
    if (version !== VERSION || !ivPart || !tagPart || !ciphertextPart) return null;
    const decipher = createDecipheriv(
      'aes-256-gcm',
      deriveKey(pollToken),
      Buffer.from(ivPart, 'base64url')
    );
    decipher.setAuthTag(Buffer.from(tagPart, 'base64url'));
    return Buffer.concat([
      decipher.update(Buffer.from(ciphertextPart, 'base64url')),
      decipher.final(),
    ]).toString('utf8');
  } catch {
    return null;
  }
}
