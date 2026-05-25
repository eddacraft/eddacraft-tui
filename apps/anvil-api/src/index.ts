import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { auth } from './routes/auth.js';
import { authDevice } from './routes/auth-device.js';
import { authOtp } from './routes/auth-otp.js';
import { authSession } from './routes/auth-session.js';
import { authGithub } from './routes/auth-github.js';
import { admin } from './routes/admin.js';
import { waitlist } from './routes/waitlist.js';
import { cron } from './routes/cron.js';
import { rateLimiter } from './middleware/rate-limit.js';
import { traceContext } from './middleware/trace-context.js';
import { getClient } from './db/client.js';
import { verifySigningKey, verifyVerifyingKey } from './lib/licence.js';

// Cold-start probe: validate both the signing-key PEM (for /device/poll and
// the OTP / GitHub / session paths) and the verifying-key PEM (for the
// requireAuth middleware on /device/confirm) parse at boot so misconfiguration
// surfaces at deploy time rather than on the first authenticated call.
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
  const [dbResult, signingKeyResult, verifyingKeyResult] = await Promise.all([
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
  ]);

  if (dbResult.ok && signingKeyResult.ok && verifyingKeyResult.ok) {
    return c.json({
      status: 'ok',
      db: 'ok',
      signingKey: 'ok',
      verifyingKey: 'ok',
    });
  }

  return c.json(
    {
      status: 'degraded',
      db: dbResult.ok ? 'ok' : 'unreachable',
      signingKey: signingKeyResult.ok ? 'ok' : 'unavailable',
      verifyingKey: verifyingKeyResult.ok ? 'ok' : 'unavailable',
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
app.route('/admin', admin);
app.route('/waitlist', waitlist);
app.route('/cron', cron);

export default app;
