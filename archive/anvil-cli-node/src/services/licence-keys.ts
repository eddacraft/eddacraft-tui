/**
 * Public keys for licence JWT verification, keyed by kid.
 *
 * Only the public key is included — it can verify signatures but not create them.
 *
 * Keys are loaded lazily from environment variables (after loadAnvilEnv() runs):
 * - LICENSE_PUBLIC_KEY_KID / LICENCE_PUBLIC_KEY_KID: the key ID (kid)
 * - LICENSE_PUBLIC_KEY / LICENCE_PUBLIC_KEY: the PEM-encoded public key
 *
 * Both US and UK spellings are accepted; US spelling takes precedence to
 * match the API's LICENSE_SIGNING_KEY convention.
 *
 * To rotate keys:
 * 1. Generate a new keypair: bash scripts/generate-licence-keypair.sh
 * 2. Configure the new public key and kid via environment variables at build time
 * 3. Ship a CLI release containing both keys (old and new)
 * 4. Update the API to sign with the new private key
 * 5. After all old licences expire (90 days), stop providing the old key
 */
export function getLicencePublicKeys(): Record<string, string> {
  const kid = process.env['LICENSE_PUBLIC_KEY_KID'] ?? process.env['LICENCE_PUBLIC_KEY_KID'];
  const key = process.env['LICENSE_PUBLIC_KEY'] ?? process.env['LICENCE_PUBLIC_KEY'];
  return kid && key ? { [kid]: key } : {};
}
