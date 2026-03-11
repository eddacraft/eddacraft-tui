/**
 * Public keys for licence JWT verification, keyed by kid.
 *
 * These are baked into the CLI at build time. Only the public key is included —
 * it can verify signatures but not create them.
 *
 * To rotate keys:
 * 1. Generate a new keypair: bash scripts/generate-licence-keypair.sh
 * 2. Add the new public key here with a new kid
 * 3. Ship a CLI release containing both keys
 * 4. Update the API to sign with the new private key
 * 5. After all old licences expire (90 days), remove the old key from here
 */
export const LICENCE_PUBLIC_KEYS: Record<string, string> = {
  // Populated from LICENSE_PUBLIC_KEY env var after keypair generation.
  // Replace this placeholder with the real PEM-encoded public key.
  //
  // Example:
  // '2026-03': `-----BEGIN PUBLIC KEY-----
  // MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE...
  // -----END PUBLIC KEY-----`,
};
