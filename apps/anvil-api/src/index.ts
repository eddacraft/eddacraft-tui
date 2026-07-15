import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { auth } from './routes/auth.js';
import { authDevice } from './routes/auth-device.js';
import { authOtp } from './routes/auth-otp.js';
import { authSession } from './routes/auth-session.js';
import { authGithub } from './routes/auth-github.js';
import { authGithubDevice } from './routes/auth-github-device.js';
import { admin } from './routes/admin.js';
import { waitlist } from './routes/waitlist.js';
import { cron } from './routes/cron.js';
import { telemetry } from './routes/telemetry.js';
import { rateLimiter } from './middleware/rate-limit.js';
import { traceContext } from './middleware/trace-context.js';
import { getClient } from './db/client.js';
import { verifySigningKey, verifyVerifyingKey } from './lib/licence.js';
import { verifyGitHubCliCredentials } from './lib/github-cli-credentials.js';
import { verifyResendKey } from './lib/resend-credentials.js';

// Cold-start probe: validate both the signing-key PEM (for /device/poll and
// the OTP / GitHub / session paths) and the verifying-key PEM (for
// verifyLicence, the licence-verification library surface — no live route
// consumes it since the #1779 confirm middleware was removed) parse at boot
// so misconfiguration surfaces at deploy time rather than on first use.
// Fire-and-forget — /health reports the result; the keys are cached via the
// module-level promises in lib/licence.ts, so this does not block request
// handling.
verifySigningKey().then((result) => {
  if (!result.ok) {
    console.error('[boot] licence signing key unavailable:', result.error);
  }
});
verifyVerifyingKey().then((result) => {
  if (!result.ok) {
    console.error('[boot] licence verifying key unavailable:', result.error);
  }
});
// GHCLIAUTH-002/-006: surface the Anvil CLI OAuth credentials at boot. The
// device-flow login is the CLI default now, so absent creds are user-impacting
// — /health reports degraded below; boot itself still completes so the
// remaining surfaces stay up.
// 2026-06-13: redeploy to pick up the Anvil CLI OAuth app re-homed under the
// eddacraft org (rotated github-cli-client-id / github-cli-client-secret in
// Key Vault); see docs/runbooks/github-device-flow.md.
{
  const result = verifyGitHubCliCredentials();
  if (!result.ok) {
    console.error('[boot] github cli credentials unavailable:', result.error);
  }
}
// CIB-067: surface a dead/missing Resend key at boot. Email senders are
// best-effort by design (and OTP request hides failures for
// anti-enumeration), so without this probe a revoked key is a silent
// production email outage — it cost 15 days once.
verifyResendKey().then((status) => {
  if (status !== 'ok') {
    console.error('[boot] resend api key not healthy:', status);
  }
});

const app = new Hono().basePath('/api/v1');

app.use('*', logger());

// CORS: restrict to configured origins, or disable for admin routes if no UI exists
const allowedOrigins: Array<string | RegExp> = (
  process.env.ANVIL_CORS_ORIGINS
    ? process.env.ANVIL_CORS_ORIGINS.split(',').map((o) => o.trim())
    : []
).map((pattern) => {
  if (pattern.includes('*')) {
    const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    return new RegExp('^' + escaped.replace(/\\\*/g, '[^.]+') + '$');
  }
  return pattern;
});

function matchOrigin(origin: string): string | undefined {
  for (const pattern of allowedOrigins) {
    if (pattern instanceof RegExp) {
      if (pattern.test(origin)) return origin;
    } else if (pattern === origin) {
      return origin;
    }
  }
  return undefined;
}

app.use(
  '*',
  cors({
    origin: (origin) => matchOrigin(origin) ?? '',
    allowMethods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowHeaders: ['Content-Type', 'Authorization', 'X-Waitlist-Admin-Token', 'traceparent'],
    // Short preflight cache: a longer TTL means an API outage poisons
    // browsers for the full TTL after recovery (a failed preflight gets
    // remembered as "no preflight allowed").
    maxAge: 300,
  })
);

app.use('*', traceContext);
app.use('*', rateLimiter());

app.get('/health', async (c) => {
  const [dbResult, signingKeyResult, verifyingKeyResult, resendKey] = await Promise.all([
    (async () => {
      try {
        const sql = getClient();
        await sql`SELECT 1`;
        return { ok: true } as const;
      } catch {
        return { ok: false } as const;
      }
    })(),
    verifySigningKey(),
    verifyVerifyingKey(),
    verifyResendKey(),
  ]);

  // GHCLIAUTH-006: the device-flow login is the CLI default, so missing CLI
  // OAuth credentials are user-impacting and gate overall health (degraded
  // 503), giving ops a pre-user-impact signal. (They were informational-only
  // under GHCLIAUTH-002, before the flow was live.)
  const githubCliCreds = verifyGitHubCliCredentials().ok ? 'ok' : 'unavailable';

  // CIB-067: a dead ('invalid') or missing ('unconfigured') Resend key means
  // invites, OTP codes, and waitlist confirmations are silently failing —
  // user-impacting, so it gates degraded. 'unverifiable' (Resend outage or
  // network failure on the probe) is reported without gating: our service is
  // not misconfigured and may recover without operator action.
  const resendGatePass = resendKey === 'ok' || resendKey === 'unverifiable';

  if (
    dbResult.ok &&
    signingKeyResult.ok &&
    verifyingKeyResult.ok &&
    githubCliCreds === 'ok' &&
    resendGatePass
  ) {
    return c.json({
      status: 'ok',
      db: 'ok',
      signingKey: 'ok',
      verifyingKey: 'ok',
      githubCliCreds,
      resendKey,
    });
  }

  return c.json(
    {
      status: 'degraded',
      db: dbResult.ok ? 'ok' : 'unreachable',
      signingKey: signingKeyResult.ok ? 'ok' : 'unavailable',
      verifyingKey: verifyingKeyResult.ok ? 'ok' : 'unavailable',
      githubCliCreds,
      resendKey,
    },
    503
  );
});

app.onError((err, c) => {
  console.error('[unhandled]', err.message, err.stack);
  return c.json({ error: 'Internal Server Error' }, 500);
});

app.route('/auth', auth);
app.route('/auth/device', authDevice);
app.route('/auth/otp', authOtp);
app.route('/auth/session', authSession);
app.route('/auth/github', authGithub);
app.route('/auth/github-device', authGithubDevice);
app.route('/admin', admin);
app.route('/waitlist', waitlist);
app.route('/cron', cron);
// FLEET-005 (ADR-107): fleet telemetry ingest. Unauthenticated by design
// (beacons come from arbitrary installs); covered by the shared '*'
// rateLimiter above and versioned by the /api/v1 base path plus the
// schema_version field in the body.
app.route('/telemetry', telemetry);

export default app;
