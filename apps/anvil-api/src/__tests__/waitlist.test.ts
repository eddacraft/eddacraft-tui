import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { waitlist } from '../routes/waitlist.js';

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
    process.env['DATABASE_URL'] = 'postgres://waitlist-test';
    process.env['WAITLIST_RESEND_ADMIN_TOKEN'] = 'waitlist-secret';
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
  });

  describe('POST /waitlist', () => {
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
