/**
 * CIB-067: Resend API key probe. Production email (beta invites, OTP codes,
 * waitlist confirmations) fails SILENTLY when the key dies — senders are
 * best-effort by design and the OTP endpoint reports success regardless to
 * avoid account enumeration. A revoked key therefore produced a 15-day
 * outage no surface reported (discovered 2026-06-12 by a pre-signup smoke).
 * This probe makes key death visible at boot and on /health, mirroring the
 * GHCLIAUTH-002 credential-probe pattern.
 */

export type ResendKeyStatus =
  | 'ok' // key accepted by the Resend API
  | 'invalid' // Resend rejected the key (401/403) — email is down, gate health
  | 'unconfigured' // RESEND_API_KEY missing — email is down, gate health
  | 'unverifiable'; // network/Resend-side failure — report, do not gate

const PROBE_TIMEOUT_MS = 5_000;
// /health is polled by uptime monitors; cache so we do not burn Resend rate
// limits or add upstream latency to every health check.
const CACHE_TTL_MS = 5 * 60 * 1_000;

let cached: { status: ResendKeyStatus; at: number } | null = null;

/**
 * Validate the configured Resend API key with a cheap authenticated read
 * (`GET /domains` — no email is sent). Results are cached for CACHE_TTL_MS.
 */
export async function verifyResendKey(): Promise<ResendKeyStatus> {
  if (cached && Date.now() - cached.at < CACHE_TTL_MS) {
    return cached.status;
  }

  const key = process.env['RESEND_API_KEY'];
  if (!key) {
    // Deliberately not cached: provisioning the env mid-flight should be
    // visible on the next check, and the check is free.
    return 'unconfigured';
  }

  let status: ResendKeyStatus;
  try {
    const res = await fetch('https://api.resend.com/domains', {
      headers: { Authorization: `Bearer ${key}` },
      signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
    });
    if (res.ok) {
      status = 'ok';
    } else if (res.status === 401 || res.status === 403) {
      // A sending-only key is rejected by read endpoints with a
      // DISTINGUISHABLE error (name: "restricted_api_key") — the key is
      // alive and can send, which is all production needs. Only a true
      // validation_error means the key is dead.
      const body = (await res.json().catch(() => null)) as { name?: string } | null;
      status = body?.name === 'restricted_api_key' ? 'ok' : 'invalid';
    } else {
      status = 'unverifiable';
    }
  } catch {
    status = 'unverifiable';
  }

  cached = { status, at: Date.now() };
  return status;
}

// Test-only: clears the probe cache so env-var changes take effect.
export function _resetResendKeyCacheForTests(): void {
  cached = null;
}
