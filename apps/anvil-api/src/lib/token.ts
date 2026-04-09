import { randomBytes, createHash } from 'node:crypto';

const TOKEN_PREFIX = 'anvil_beta_';
// base64url(32 bytes) = 43 characters (no padding needed for 32 bytes)
const TOKEN_PAYLOAD_LENGTH = 43;

// Fail fast at cold start rather than during request handling.
// In serverless (Vercel), module-level code runs once per instance.
if (process.env['NODE_ENV'] === 'production' && !process.env['TOKEN_PEPPER']) {
  throw new Error('TOKEN_PEPPER environment variable is required in production');
}

/**
 * Generate a new beta access token.
 * Format: anvil_beta_<base64url(32 random bytes)>
 */
export function generateToken(): string {
  const bytes = randomBytes(32);
  const payload = bytes.toString('base64url');
  return `${TOKEN_PREFIX}${payload}`;
}

/**
 * Hash a raw token for storage.
 * Uses SHA-256 with a pepper from environment.
 */
export function hashToken(raw: string): string {
  const pepper = process.env['TOKEN_PEPPER'] ?? '';
  return createHash('sha256')
    .update(pepper + raw)
    .digest('hex');
}

/**
 * Validate that a string matches the expected token format.
 */
export function isValidTokenFormat(token: string): boolean {
  if (!token.startsWith(TOKEN_PREFIX)) return false;
  const payload = token.slice(TOKEN_PREFIX.length);
  // base64url: A-Z, a-z, 0-9, -, _
  return new RegExp(`^[A-Za-z0-9_-]{${TOKEN_PAYLOAD_LENGTH}}$`).test(payload);
}
