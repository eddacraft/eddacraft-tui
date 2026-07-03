import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { waitlist } from '../routes/waitlist.js';
import { waitlistEmailThrottle } from '../middleware/waitlist-throttle.js';

const waitlistMocks = vi.hoisted(() => ({
  getClient: vi.fn(),
  sql: vi.fn(),
  sendWaitlistConfirmation: vi.fn(),
  sendWaitlistAdminNotification: vi.fn(),
}));

vi.mock('../db/client.js', () => ({
  getClient: waitlistMocks.getClient,
}));

vi.mock('../lib/email.js', () => ({
  sendWaitlistConfirmation: waitlistMocks.sendWaitlistConfirmation,
  sendWaitlistAdminNotification: waitlistMocks.sendWaitlistAdminNotification,
}));

afterEach(() => {
  vi.restoreAllMocks();
});

const app = new Hono();
app.route('/waitlist', waitlist);

const originalDatabaseUrl = process.env['DATABASE_URL'];
const originalAdminToken = process.env['WAITLIST_RESEND_ADMIN_TOKEN'];
const originalWaitlistPaused = process.env['WAITLIST_PAUSED'];

function request(path: string, body: BodyInit, headers: HeadersInit = {}) {
  return app.request(path, {
    method: 'POST',
    headers,
    body,
  });
}

describe('waitlist routes', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Reset the shared per-email throttle so submissions in one test don't
    // spend another test's budget (the throttle store is process-wide).
    waitlistEmailThrottle.reset();
    process.env['DATABASE_URL'] = 'postgres://waitlist-test';
    process.env['WAITLIST_RESEND_ADMIN_TOKEN'] = 'waitlist-secret';
    delete process.env['WAITLIST_PAUSED'];
    waitlistMocks.getClient.mockReturnValue(waitlistMocks.sql);
  });

  afterEach(() => {
    if (originalDatabaseUrl === undefined) {
      delete process.env['DATABASE_URL'];
    } else {
      process.env['DATABASE_URL'] = originalDatabaseUrl;
    }

    if (originalAdminToken === undefined) {
      delete process.env['WAITLIST_RESEND_ADMIN_TOKEN'];
    } else {
      process.env['WAITLIST_RESEND_ADMIN_TOKEN'] = originalAdminToken;
    }

    if (originalWaitlistPaused === undefined) {
      delete process.env['WAITLIST_PAUSED'];
    } else {
      process.env['WAITLIST_PAUSED'] = originalWaitlistPaused;
    }
  });

  describe('POST /waitlist', () => {
    it('returns 503 and rejects signups when WAITLIST_PAUSED=true', async () => {
      process.env['WAITLIST_PAUSED'] = 'true';

      const response = await request('/waitlist', JSON.stringify({ email: 'person@example.com' }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(503);
      expect(await response.json()).toEqual({
        error: 'Waitlist temporarily paused for maintenance',
      });
      expect(waitlistMocks.getClient).not.toHaveBeenCalled();
      expect(waitlistMocks.sendWaitlistConfirmation).not.toHaveBeenCalled();
    });

    it('ignores WAITLIST_PAUSED values other than "true"', async () => {
      process.env['WAITLIST_PAUSED'] = '1';
      waitlistMocks.sql.mockResolvedValue([
        {
          id: 1,
          email: 'person@example.com',
          created_at: '2026-03-14T00:00:00.000Z',
          is_new: false,
        },
      ]);

      const response = await request('/waitlist', JSON.stringify({ email: 'person@example.com' }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(200);
    });

    it('adds a new signup and sends a confirmation email', async () => {
      waitlistMocks.sql.mockResolvedValue([
        {
          id: 1,
          email: 'person@example.com',
          created_at: '2026-03-14T00:00:00.000Z',
          is_new: true,
        },
      ]);
      waitlistMocks.sendWaitlistConfirmation.mockResolvedValue({ sent: true });

      const response = await request(
        '/waitlist',
        JSON.stringify({ email: ' Person@Example.com ' }),
        {
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(200);
      expect(waitlistMocks.getClient).toHaveBeenCalledTimes(1);
      expect(waitlistMocks.sendWaitlistConfirmation).toHaveBeenCalledWith('person@example.com');
      expect(await response.json()).toEqual({
        success: true,
        message: 'Added to waitlist',
        email: 'person@example.com',
        isNewSignup: true,
        emailSent: true,
        emailStatus: 'sent',
      });
    });

    it('accepts an existing signup without sending another email', async () => {
      waitlistMocks.sql.mockResolvedValue([
        {
          id: 1,
          email: 'person@example.com',
          created_at: '2026-03-14T00:00:00.000Z',
          is_new: false,
        },
      ]);

      const response = await request('/waitlist', JSON.stringify({ email: 'person@example.com' }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(200);
      expect(waitlistMocks.sendWaitlistConfirmation).not.toHaveBeenCalled();
      expect(await response.json()).toEqual({
        success: true,
        message: 'Added to waitlist',
        email: 'person@example.com',
        isNewSignup: false,
        emailSent: false,
        emailStatus: 'skipped',
      });
    });

    it('returns email delivery metadata when confirmation sending fails', async () => {
      waitlistMocks.sql.mockResolvedValue([
        {
          id: 1,
          email: 'person@example.com',
          created_at: '2026-03-14T00:00:00.000Z',
          is_new: true,
        },
      ]);
      waitlistMocks.sendWaitlistConfirmation.mockResolvedValue({
        sent: false,
        code: 'provider_error',
        message: 'Provider unavailable',
      });

      const response = await request('/waitlist', JSON.stringify({ email: 'person@example.com' }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(200);
      expect(await response.json()).toEqual({
        success: true,
        message: 'Added to waitlist',
        email: 'person@example.com',
        isNewSignup: true,
        emailSent: false,
        emailStatus: 'provider_error',
      });
    });

    it('rejects requests without an application/json content type', async () => {
      const response = await request('/waitlist', '{"email":"person@example.com"}', {
        'Content-Type': 'text/plain',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Content-Type must be application/json' });
    });

    it('rejects malformed JSON payloads', async () => {
      const response = await request('/waitlist', '{', {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Invalid JSON payload' });
    });

    it.each([
      {
        name: 'non-object payloads',
        body: JSON.stringify(['person@example.com']),
        expected: { error: 'Invalid JSON payload' },
      },
      {
        name: 'missing email fields',
        body: JSON.stringify({}),
        expected: { error: 'Email is required' },
      },
      {
        name: 'invalid email addresses',
        body: JSON.stringify({ email: 'not-an-email' }),
        expected: { error: 'Invalid email format' },
      },
    ])('rejects $name', async ({ body, expected }) => {
      const response = await request('/waitlist', body, {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual(expected);
    });

    it('rejects emails longer than 254 characters without calling the DB', async () => {
      const longLocal = 'a'.repeat(245);
      const longEmail = `${longLocal}@example.com`; // 245 + 12 = 257 chars

      const response = await request('/waitlist', JSON.stringify({ email: longEmail }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Invalid email format' });
      expect(waitlistMocks.getClient).not.toHaveBeenCalled();
    });

    it('returns 503 when DATABASE_URL is not configured', async () => {
      delete process.env['DATABASE_URL'];

      const response = await request('/waitlist', JSON.stringify({ email: 'person@example.com' }), {
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(503);
      expect(await response.json()).toEqual({ error: 'Service unavailable' });
      expect(waitlistMocks.getClient).not.toHaveBeenCalled();
    });

    describe('per-email abuse throttle', () => {
      // The shared throttle default is 3 submissions per email per window.
      const THROTTLE_MAX = 3;

      function existingEntry(email: string) {
        return [{ id: 1, email, created_at: '2026-03-14T00:00:00.000Z', is_new: false }];
      }

      it('throttles repeated submissions for the same email independent of source IP', async () => {
        waitlistMocks.sql.mockResolvedValue(existingEntry('bombme@example.com'));

        const statuses: number[] = [];
        for (let i = 0; i < THROTTLE_MAX + 2; i++) {
          const response = await request(
            '/waitlist',
            JSON.stringify({ email: 'bombme@example.com' }),
            {
              'Content-Type': 'application/json',
              // A different, valid client IP on every request: the per-email
              // throttle must fire regardless of source IP.
              'x-real-ip': `203.0.113.${i + 1}`,
            }
          );
          statuses.push(response.status);
        }

        // First THROTTLE_MAX pass; every submission after the cap is 429.
        expect(statuses.slice(0, THROTTLE_MAX)).toEqual([200, 200, 200]);
        expect(statuses.slice(THROTTLE_MAX)).toEqual([429, 429]);
      });

      it('returns the shared limiter error shape and a Retry-After header when throttled', async () => {
        waitlistMocks.sql.mockResolvedValue(existingEntry('bombme@example.com'));

        let last: Response | undefined;
        for (let i = 0; i < THROTTLE_MAX + 1; i++) {
          last = await request('/waitlist', JSON.stringify({ email: 'bombme@example.com' }), {
            'Content-Type': 'application/json',
          });
        }

        expect(last?.status).toBe(429);
        expect(await last?.json()).toEqual({ error: 'Too many requests, please try again later' });
        expect(Number(last?.headers.get('Retry-After'))).toBeGreaterThan(0);
      });

      it('does not throttle submissions for different emails', async () => {
        waitlistMocks.sql.mockResolvedValue(existingEntry('someone@example.com'));

        const statuses: number[] = [];
        for (let i = 0; i < THROTTLE_MAX + 2; i++) {
          const response = await request(
            '/waitlist',
            JSON.stringify({ email: `person${i}@example.com` }),
            { 'Content-Type': 'application/json' }
          );
          statuses.push(response.status);
        }

        expect(statuses.every((s) => s === 200)).toBe(true);
      });

      it('shares one bucket across case and whitespace variants of the same email', async () => {
        waitlistMocks.sql.mockResolvedValue(existingEntry('person@example.com'));

        const variants = [
          'person@example.com',
          'Person@Example.com',
          '  PERSON@example.com  ',
          'person@example.com',
        ];

        const statuses: number[] = [];
        for (const email of variants) {
          const response = await request('/waitlist', JSON.stringify({ email }), {
            'Content-Type': 'application/json',
          });
          statuses.push(response.status);
        }

        // Three variants map to one bucket (cap 3); the fourth is throttled.
        expect(statuses).toEqual([200, 200, 200, 429]);
      });
    });
  });

  describe('POST /waitlist/resend', () => {
    it('rejects unauthorised resend requests', async () => {
      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'person@example.com' }),
        {
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(401);
      expect(await response.json()).toEqual({ error: 'Unauthorized' });
    });

    it('rejects Bearer tokens that do not match the admin secret', async () => {
      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'person@example.com' }),
        {
          Authorization: 'Bearer wrong-secret',
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(401);
      expect(await response.json()).toEqual({ error: 'Unauthorized' });
      expect(waitlistMocks.sendWaitlistConfirmation).not.toHaveBeenCalled();
    });

    it('rejects requests when WAITLIST_RESEND_ADMIN_TOKEN is unset', async () => {
      delete process.env['WAITLIST_RESEND_ADMIN_TOKEN'];

      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'person@example.com' }),
        {
          Authorization: 'Bearer anything',
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(401);
      expect(await response.json()).toEqual({ error: 'Unauthorized' });
      expect(waitlistMocks.sendWaitlistConfirmation).not.toHaveBeenCalled();
    });

    it('rejects resend requests missing the email field', async () => {
      const response = await request('/waitlist/resend', JSON.stringify({}), {
        Authorization: 'Bearer waitlist-secret',
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Email is required' });
      expect(waitlistMocks.sendWaitlistConfirmation).not.toHaveBeenCalled();
    });

    it('authorises Bearer token requests and sends the email', async () => {
      waitlistMocks.sendWaitlistConfirmation.mockResolvedValue({ sent: true });

      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'Person@Example.com' }),
        {
          Authorization: 'Bearer waitlist-secret',
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(200);
      expect(waitlistMocks.sendWaitlistConfirmation).toHaveBeenCalledWith('person@example.com');
      expect(await response.json()).toEqual({
        success: true,
        email: 'person@example.com',
        emailSent: true,
        emailStatus: 'sent',
      });
    });

    it('authorises x-waitlist-admin-token requests and sends the email', async () => {
      waitlistMocks.sendWaitlistConfirmation.mockResolvedValue({ sent: true });

      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'person@example.com' }),
        {
          'Content-Type': 'application/json',
          'x-waitlist-admin-token': 'waitlist-secret',
        }
      );

      expect(response.status).toBe(200);
      expect(waitlistMocks.sendWaitlistConfirmation).toHaveBeenCalledWith('person@example.com');
      expect(await response.json()).toEqual({
        success: true,
        email: 'person@example.com',
        emailSent: true,
        emailStatus: 'sent',
      });
    });

    it('rejects malformed resend payloads', async () => {
      const response = await request('/waitlist/resend', '{', {
        Authorization: 'Bearer waitlist-secret',
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Invalid JSON payload' });
    });

    it('rejects resend requests with invalid email addresses', async () => {
      const response = await request('/waitlist/resend', JSON.stringify({ email: 'invalid' }), {
        Authorization: 'Bearer waitlist-secret',
        'Content-Type': 'application/json',
      });

      expect(response.status).toBe(400);
      expect(await response.json()).toEqual({ error: 'Invalid email format' });
    });

    it('returns a 502 response when email delivery fails', async () => {
      waitlistMocks.sendWaitlistConfirmation.mockResolvedValue({
        sent: false,
        code: 'provider_error',
        message: 'Provider unavailable',
      });

      const response = await request(
        '/waitlist/resend',
        JSON.stringify({ email: 'person@example.com' }),
        {
          Authorization: 'Bearer waitlist-secret',
          'Content-Type': 'application/json',
        }
      );

      expect(response.status).toBe(502);
      expect(await response.json()).toEqual({
        success: false,
        email: 'person@example.com',
        emailSent: false,
        emailStatus: 'provider_error',
        error: 'Provider unavailable',
      });
    });
  });
});
