import { Hono } from 'hono';
import { timingSafeEqual } from 'node:crypto';
import { getClient } from '../db/client.js';
import { upsertWaitlistEntry } from '../db/queries.js';
import { sendWaitlistConfirmation, sendWaitlistAdminNotification } from '../lib/email.js';
import { addToWaitlistAudience } from '../lib/audience.js';
import { waitlistEmailThrottle } from '../middleware/waitlist-throttle.js';

export const waitlist = new Hono();

const EMAIL_REGEX =
  /^[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+)*@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*\.[a-zA-Z]{2,}$/;

waitlist.post('/', async (c) => {
  try {
    // DBCON-003: WAITLIST_PAUSED short-circuits new signups during the Neon
    // consolidation cutover so the delta sync can't miss rows that only land
    // in the source DB. Set via Vercel env; changes require an anvil-api
    // redeploy to take effect (env vars are baked into each deployment).
    if (process.env.WAITLIST_PAUSED === 'true') {
      return c.json({ error: 'Waitlist temporarily paused for maintenance' }, 503);
    }

    const databaseUrl = process.env.DATABASE_URL;
    if (!databaseUrl) {
      console.error('DATABASE_URL not configured');
      return c.json({ error: 'Service unavailable' }, 503);
    }

    const contentType = c.req.header('content-type') ?? '';
    if (!contentType.toLowerCase().includes('application/json')) {
      return c.json({ error: 'Content-Type must be application/json' }, 400);
    }

    let body: unknown;
    try {
      body = await c.req.json();
    } catch {
      return c.json({ error: 'Invalid JSON payload' }, 400);
    }

    if (!body || typeof body !== 'object' || Array.isArray(body)) {
      return c.json({ error: 'Invalid JSON payload' }, 400);
    }

    const { email } = body as { email?: unknown };
    if (!email || typeof email !== 'string') {
      return c.json({ error: 'Email is required' }, 400);
    }

    const trimmedEmail = email.trim();
    if (trimmedEmail.length > 254) {
      return c.json({ error: 'Invalid email format' }, 400);
    }
    if (!EMAIL_REGEX.test(trimmedEmail)) {
      return c.json({ error: 'Invalid email format' }, 400);
    }

    const normalisedEmail = trimmedEmail.toLowerCase();

    // Per-email abuse throttle, independent of source IP. The global limiter
    // (rateLimiter, CIB-140) keys on the Vercel client IP and cannot stop
    // signup abuse / email-bombing of one mailbox from many IPs; this closes
    // that gap by throttling repeated submissions for the same address.
    // Successful and failed submissions both count. Best-effort / in-memory,
    // consistent with the global limiter's posture (see waitlist-throttle.ts).
    const throttle = waitlistEmailThrottle.consume(normalisedEmail);
    if (throttle.limited) {
      c.res.headers.set('Retry-After', String(throttle.retryAfterSeconds));
      return c.json({ error: 'Too many requests, please try again later' }, 429);
    }

    const sql = getClient();
    const entry = await upsertWaitlistEntry(sql, normalisedEmail);

    const isNewSignup = entry.is_new;
    let emailSent = false;
    let emailStatus = 'skipped';

    if (isNewSignup) {
      const audienceUpdate = addToWaitlistAudience(entry.email);
      try {
        c.executionCtx.waitUntil(audienceUpdate);
      } catch {
        void audienceUpdate;
      }
      const delivery = await sendWaitlistConfirmation(entry.email);
      emailSent = delivery.sent;
      emailStatus = delivery.sent ? 'sent' : (delivery.code ?? 'failed');
    }

    // Await rather than fire-and-forget: on Vercel Node serverless,
    // c.executionCtx is unavailable and the catch fallback lets the
    // lambda freeze before the Resend HTTP call flushes.
    await sendWaitlistAdminNotification(entry.email, isNewSignup, emailSent);

    return c.json({
      success: true,
      message: 'Added to waitlist',
      email: entry.email,
      isNewSignup,
      emailSent,
      emailStatus,
    });
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error('Waitlist submission error:', error.message);
    } else {
      console.error('Waitlist submission error:', error);
    }
    return c.json({ error: 'Failed to join waitlist' }, 500);
  }
});

waitlist.post('/resend', async (c) => {
  const expectedToken = process.env.WAITLIST_RESEND_ADMIN_TOKEN;
  if (!expectedToken) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  const authHeader = c.req.header('authorization');
  const bearer = authHeader?.startsWith('Bearer ') ? authHeader.slice(7).trim() : null;
  const direct = c.req.header('x-waitlist-admin-token')?.trim();

  const token = bearer ?? direct;
  if (!token) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  const a = Buffer.from(expectedToken, 'utf-8');
  const b = Buffer.from(token, 'utf-8');
  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'Invalid JSON payload' }, 400);
  }

  if (!body || typeof body !== 'object' || Array.isArray(body)) {
    return c.json({ error: 'Invalid JSON payload' }, 400);
  }

  const { email } = body as { email?: unknown };
  if (!email || typeof email !== 'string') {
    return c.json({ error: 'Email is required' }, 400);
  }

  const trimmedEmail = email.trim().toLowerCase();
  if (trimmedEmail.length > 254 || !EMAIL_REGEX.test(trimmedEmail)) {
    return c.json({ error: 'Invalid email format' }, 400);
  }

  const delivery = await sendWaitlistConfirmation(trimmedEmail);

  if (!delivery.sent) {
    return c.json(
      {
        success: false,
        email: trimmedEmail,
        emailSent: false,
        emailStatus: delivery.code ?? 'failed',
        error: delivery.message ?? 'Failed to send confirmation email',
      },
      502
    );
  }

  return c.json({
    success: true,
    email: trimmedEmail,
    emailSent: true,
    emailStatus: 'sent',
  });
});
