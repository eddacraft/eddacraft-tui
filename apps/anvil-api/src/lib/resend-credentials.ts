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
let inFlight: Promise<ResendKeyStatus> | null = null;

/**
 * Validate the configured Resend API key with a cheap authenticated read
 * (`GET /domains` — no email is sent). Stale-while-revalidate: an expired
 * cache entry is served immediately while a background refresh runs, so
 * /health never pays the upstream round-trip after the boot probe has
 * primed the cache. Concurrent cache misses share one in-flight request.
 */
export async function verifyResendKey(): Promise<ResendKeyStatus> {
  if (cached) {
    if (Date.now() - cached.at >= CACHE_TTL_MS) {
      // Background refresh shares the same in-flight slot as cold misses,
      // so concurrent stale hits cannot stampede Resend either.
      void startRefresh().catch(() => {});
    }
    return cached.status;
  }
  return startRefresh();
}

function startRefresh(): Promise<ResendKeyStatus> {
  if (!inFlight) {
    inFlight = refresh().finally(() => {
      inFlight = null;
    });
  }
  return inFlight;
}

async function refresh(): Promise<ResendKeyStatus> {
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
      // Resend's 401/403 bodies carry a discriminating `name`:
      // - restricted_api_key: a sending-only key rejected by a read
      //   endpoint — the key is ALIVE and can send, which is all
      //   production email needs → ok.
      // - validation_error / invalid_api_key: the key is dead → invalid.
      // - anything unrecognised (schema change, parse failure): the probe
      //   cannot conclude — report unverifiable rather than gate a 503 on
      //   a guess. (This module exists because of a Resend key surprise;
      //   don't let a renamed error field cause the next one.)
      const body = (await res.json().catch(() => null)) as { name?: string } | null;
      if (body?.name === 'restricted_api_key') {
        status = 'ok';
      } else if (body?.name === 'validation_error' || body?.name === 'invalid_api_key') {
        status = 'invalid';
      } else {
        status = 'unverifiable';
      }
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
  inFlight = null;
}
