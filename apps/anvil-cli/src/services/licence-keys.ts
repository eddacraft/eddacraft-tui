/**
 * Public keys for licence JWT verification, keyed by kid.
 *
 * These are baked into the CLI at build time. Only the public key is included —
 * it can verify signatures but not create them.
 *
 * The primary key is loaded from environment variables:
 * - LICENCE_PUBLIC_KEY_KID: the key ID (kid) used by the API when signing
 * - LICENCE_PUBLIC_KEY: the PEM-encoded public key corresponding to that kid
 *
 * Build/deploy pipelines should set these so that LICENCE_PUBLIC_KEYS is
 * populated with at least one real key in production.
 *
 * To rotate keys:
 * 1. Generate a new keypair: bash scripts/generate-licence-keypair.sh
 * 2. Configure the new public key and kid via environment variables at build time
 * 3. Ship a CLI release containing both keys (old and new)
 * 4. Update the API to sign with the new private key
 * 5. After all old licences expire (90 days), stop providing the old key
 */
const licencePublicKeyKid = process.env['LICENCE_PUBLIC_KEY_KID'];
const licencePublicKey = process.env['LICENCE_PUBLIC_KEY'];

export const LICENCE_PUBLIC_KEYS: Record<string, string> =
  licencePublicKeyKid && licencePublicKey ? { [licencePublicKeyKid]: licencePublicKey } : {};
