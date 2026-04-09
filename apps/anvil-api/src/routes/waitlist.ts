import { Hono } from 'hono';
import { timingSafeEqual } from 'node:crypto';
import { getClient } from '../db/client.js';
import { sendWaitlistConfirmation, sendWaitlistAdminNotification } from '../lib/email.js';
import { addToWaitlistAudience } from '../lib/audience.js';

export const waitlist = new Hono();

const EMAIL_REGEX =
  /^[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+)*@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*\.[a-zA-Z]{2,}$/;

waitlist.post('/', async (c) => {
  try {
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

    const sql = getClient();
    const result = (await sql`
      INSERT INTO waitlist (email, source)
      VALUES (${trimmedEmail.toLowerCase()}, 'website')
      ON CONFLICT (email) DO UPDATE SET updated_at = NOW()
      RETURNING id, email, created_at, (xmax = 0) AS is_new
    `) as { id: number; email: string; created_at: string; is_new: boolean }[];

    if (!Array.isArray(result) || result.length === 0) {
      console.error('Waitlist insertion did not return a result');
      return c.json({ error: 'Failed to join waitlist' }, 500);
    }

    const isNewSignup = result[0].is_new;
    let emailSent = false;
    let emailStatus = 'skipped';

    if (isNewSignup) {
      const audienceUpdate = addToWaitlistAudience(result[0].email);
      try {
        c.executionCtx.waitUntil(audienceUpdate);
      } catch {
        void audienceUpdate;
      }
      const delivery = await sendWaitlistConfirmation(result[0].email);
      emailSent = delivery.sent;
      emailStatus = delivery.sent ? 'sent' : (delivery.code ?? 'failed');
    }

    const adminNotification = sendWaitlistAdminNotification(
      result[0].email,
      isNewSignup,
      emailSent
    );
    try {
      c.executionCtx.waitUntil(adminNotification);
    } catch {
      void adminNotification;
    }

    return c.json({
      success: true,
      message: 'Added to waitlist',
      email: result[0].email,
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
