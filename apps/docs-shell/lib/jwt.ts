import { jwtVerify, importSPKI, type CryptoKey } from 'jose';

let cachedKey: CryptoKey | null = null;
/**
 * Plans entitled to private `/anvil` docs. Mirrors the `docs.access` targeting
 * in `flags/manifest.json` (`accountTier in_set [plan-beta, plan-pro,
 * plan-enterprise]`), expressed as bare plan names because this verifier reads
 * the raw JWT at the edge with no catalogue import. Keep the two in step; see
 * SEC-012's residual note about sourcing this from the catalogue directly.
 */
const DOCS_ACCESS_PLANS = new Set(['beta', 'pro', 'enterprise']);

/**
 * SEC-012: the one legacy `tier` value ever minted, and the plan it
 * de-escalates to. Pre-BACT-013 `signLicence` hardcoded `tier: 'pro'` for
 * every licence and wrote no `plan`. No account has ever held `pro`, and this
 * verifier does no DB round trip, so inheriting the claim would let a stale
 * token assert a plan nobody has. Matching the value exactly (rather than
 * de-escalating any `tier`) keeps a value we never issued denied instead of
 * being handed `beta`. Delete once the last pre-BACT-013 licence expires
 * (90-day TTL from 2026-08-13 => ~2026-11-11).
 */
const LEGACY_TIER_VALUE = 'pro';
const STALE_TIER_PLAN = 'beta';

/**
 * Resolve the entitlement claim without letting a token elevate itself.
 * Mirrors `verifyLicence` in `apps/anvil-api/src/lib/licence.ts` — the two
 * must agree, because they gate the same product decision from different
 * sides. Returns null when no claim we trust is present; the caller denies.
 */
function resolvePlanClaim(payload: Record<string, unknown>): string | null {
  const rawPlan = payload['plan'];
  if (typeof rawPlan === 'string') return rawPlan;
  if (payload['tier'] === LEGACY_TIER_VALUE) return STALE_TIER_PLAN;
  return null;
}
/** Must match anvil-api `signLicence` (apps/anvil-api/src/lib/licence.ts). */
const LICENCE_ISSUER = 'https://api.eddacraft.ai';
const LICENCE_AUDIENCE = 'anvil-cli';

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
  /**
   * The entitlement plan this licence resolved to, or null when it carried no
   * claim we trust. Exposed so the de-escalation is observable (and testable)
   * rather than an invisible side effect of the valid/invalid verdict.
   */
  plan?: string | null;
}

export async function verifyLicense(token: string): Promise<VerifyResult> {
  try {
    const publicKey = await getPublicKey();
    const { payload } = await jwtVerify(token, publicKey, {
      algorithms: ['ES256'],
      issuer: LICENCE_ISSUER,
      audience: LICENCE_AUDIENCE,
    });
    if (typeof payload.sub !== 'string' || payload.sub.length === 0) {
      return { valid: false };
    }
    const plan = resolvePlanClaim(payload as Record<string, unknown>);
    if (plan === null || !DOCS_ACCESS_PLANS.has(plan)) {
      return { valid: false, plan };
    }
    return { valid: true, plan };
  } catch {
    return { valid: false };
  }
}
