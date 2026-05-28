import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';
import { _resetAdminRateLimitForTests } from '../middleware/admin-rate-limit.js';

vi.mock('../lib/feature-flags.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/feature-flags.js')>();
  return {
    ...actual,
    resolveApiScope: vi.fn(actual.resolveApiScope),
  };
});

afterEach(() => {
  vi.restoreAllMocks();
});

const ADMIN_KEY = 'test-admin-key-12345';

function createMockSql() {
  const sql = vi.fn() as ReturnType<typeof vi.fn> & { transaction: ReturnType<typeof vi.fn> };
  sql.transaction = vi.fn();
  return sql;
}

const mockSql = createMockSql();

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => mockSql),
}));

vi.mock('../db/queries.js', () => ({
  insertAuditLog: vi.fn().mockResolvedValue({
    id: 'audit-1',
    action: '',
    actor: '',
    metadata: {},
    created_at: new Date().toISOString(),
  }),
  insertBroadcastSnapshot: vi.fn(),
  findBroadcastSnapshot: vi.fn().mockResolvedValue(null),
  consumeBroadcastSnapshot: vi.fn().mockResolvedValue(null),
  findAdminKeyByHash: vi.fn().mockResolvedValue(null),
  // Other queries the broadcast route never calls but the admin route module
  // re-exports — keep them as no-ops so the route import resolves.
  findUserByEmail: vi.fn(),
  findUserWithTokens: vi.fn(),
  upsertWaitlistWithName: vi.fn().mockResolvedValue(undefined),
  findWaitlistEntryByEmail: vi.fn().mockResolvedValue({ id: '1' }),
  findUnapprovedWaitlistEntries: vi.fn().mockResolvedValue([]),
  findWaitlistBySource: vi.fn().mockResolvedValue([]),
  findWaitlistPaginated: vi.fn().mockResolvedValue({ total: 0, items: [] }),
  findAuditEntries: vi.fn().mockResolvedValue({ total: 0, items: [] }),
  findRecentAuditForEmail: vi.fn().mockResolvedValue([]),
  findActiveScopesForUser: vi.fn().mockResolvedValue(['beta']),
}));

vi.mock('../lib/token.js', () => ({
  generateToken: vi.fn().mockReturnValue('anvil_beta_' + 'X'.repeat(43)),
  hashToken: vi.fn().mockReturnValue('mocked-hash'),
  isValidTokenFormat: vi.fn().mockReturnValue(true),
}));

vi.mock('../lib/email.js', () => ({
  sendBetaInvite: vi.fn().mockResolvedValue({ sent: true }),
  sendWaitlistMigration: vi.fn().mockResolvedValue({ sent: true }),
  sendReleaseAnnouncement: vi.fn().mockResolvedValue({ sent: true }),
}));

vi.mock('../lib/audience.js', () => ({
  moveToApprovedAudience: vi.fn().mockResolvedValue(undefined),
  removeFromBetaAudience: vi.fn().mockResolvedValue(undefined),
}));

// The broadcast route resolves audience rows via this module. The mock
// returns whatever the test queues up via mockResolveAudience.
const resolveAudienceMock = vi.hoisted(() => vi.fn());
vi.mock('../lib/broadcast-audiences.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/broadcast-audiences.js')>();
  return {
    ...actual,
    resolveAudience: resolveAudienceMock,
  };
});

import {
  insertAuditLog,
  insertBroadcastSnapshot,
  findBroadcastSnapshot,
  consumeBroadcastSnapshot,
} from '../db/queries.js';
import { sendReleaseAnnouncement, sendWaitlistMigration } from '../lib/email.js';

const app = new Hono();
app.route('/admin', admin);

function request(method: string, path: string, body?: unknown, authKey?: string) {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (authKey) headers['Authorization'] = `Bearer ${authKey}`;
  return app.request(path, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
}

describe('POST /admin/broadcast', () => {
  const originalAdminKey = process.env['ADMIN_KEY'];

  beforeEach(() => {
    vi.clearAllMocks();
    _resetAdminRateLimitForTests();
    process.env['ADMIN_KEY'] = ADMIN_KEY;
    resolveAudienceMock.mockResolvedValue([]);
  });

  afterEach(() => {
    if (originalAdminKey === undefined) {
      delete process.env['ADMIN_KEY'];
    } else {
      process.env['ADMIN_KEY'] = originalAdminKey;
    }
  });

  describe('input validation', () => {
    it('rejects unknown templates with 400 template_unknown', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        { template: 'no-such-template', audience: 'beta:active', dryRun: true },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('template_unknown');
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects transactional templates with 400 template_kind_not_broadcastable', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        { template: 'otp-code', audience: 'beta:active', dryRun: true },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('template_kind_not_broadcastable');
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects unknown audiences with 400 audience_unknown', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        { template: 'release-announcement', audience: 'no-such-audience', dryRun: true },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('audience_unknown');
    });

    it('rejects waitlist:source without audienceParams.source with 400 audience_params_missing', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        { template: 'release-announcement', audience: 'waitlist:source', dryRun: true },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('audience_params_missing');
    });

    it('rejects invalid templateProps with 400 template_props_invalid', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: true,
          // strict schema rejects unknown keys
          templateProps: { unknownField: 'oops' },
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('template_props_invalid');
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects audienceParams with > 16 keys', async () => {
      const tooMany: Record<string, string> = {};
      for (let i = 0; i < 17; i++) tooMany[`k${i}`] = 'v';
      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          audienceParams: tooMany,
          dryRun: true,
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects audienceParams with an oversized value', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          audienceParams: { source: 'x'.repeat(1100) },
          dryRun: true,
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects templateProps with > 64 keys', async () => {
      const tooMany: Record<string, unknown> = {};
      for (let i = 0; i < 65; i++) tooMany[`k${i}`] = 'v';
      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          templateProps: tooMany,
          dryRun: true,
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });
  });

  describe('dry-run', () => {
    const recipients = [
      { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
      { email: 'bob@example.com', name: null, user_id: 'u-2' },
    ];

    function makeSnapshot(overrides: Partial<Record<string, unknown>> = {}) {
      return {
        token: 'snap-bc-abc',
        template: 'release-announcement',
        template_props: {},
        audience_key: 'beta:active',
        audience_params: {},
        recipients: recipients.map(({ email, name }) => ({ email, name })),
        created_by_actor: 'shared-key@anvil',
        created_at: '2026-05-24T09:00:00Z',
        expires_at: '2026-05-24T09:10:00Z',
        consumed_at: null,
        ...overrides,
      };
    }

    it('returns recipients, previewToken, expiresAt, and the parsed templateProps', async () => {
      resolveAudienceMock.mockResolvedValueOnce(recipients);
      vi.mocked(insertBroadcastSnapshot).mockResolvedValueOnce(makeSnapshot());

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: true,
        },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.dryRun).toBe(true);
      expect(body.template).toBe('release-announcement');
      expect(body.audience).toBe('beta:active');
      expect(body.count).toBe(2);
      expect(body.recipients).toEqual([
        { email: 'alice@example.com', name: 'Alice' },
        { email: 'bob@example.com', name: null },
      ]);
      expect(body.previewToken).toBe('snap-bc-abc');
      expect(body.expiresAt).toBe('2026-05-24T09:10:00Z');
    });

    it('snapshots template, templateProps, audienceKey, and audienceParams', async () => {
      resolveAudienceMock.mockResolvedValueOnce(recipients);
      vi.mocked(insertBroadcastSnapshot).mockResolvedValueOnce(makeSnapshot());

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'waitlist:source',
          audienceParams: { source: 'import' },
          templateProps: { version: 'v0.8.0-beta', theme: 'Test theme' },
          dryRun: true,
        },
        ADMIN_KEY
      );

      const insertCall = vi.mocked(insertBroadcastSnapshot).mock.calls[0]?.[1];
      expect(insertCall).toMatchObject({
        template: 'release-announcement',
        templateProps: { version: 'v0.8.0-beta', theme: 'Test theme' },
        audienceKey: 'waitlist:source',
        audienceParams: { source: 'import' },
        createdByActor: 'shared-key@anvil',
        ttlSeconds: 600,
      });
      expect(typeof insertCall?.token).toBe('string');
      expect(insertCall?.token.length).toBeGreaterThanOrEqual(16);
    });

    it('resolves audience with the request limit', async () => {
      resolveAudienceMock.mockResolvedValueOnce([]);
      vi.mocked(insertBroadcastSnapshot).mockResolvedValueOnce(makeSnapshot({ recipients: [] }));

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active-recent',
          limit: 42,
          dryRun: true,
        },
        ADMIN_KEY
      );

      expect(resolveAudienceMock).toHaveBeenCalledWith(
        expect.anything(),
        'beta:active-recent',
        expect.objectContaining({ limit: 42 })
      );
    });

    it('rejects limit > 80 via zod with 400 (synchronous loop survival cap)', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          limit: 1000,
          dryRun: true,
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect(resolveAudienceMock).not.toHaveBeenCalled();
    });

    it('defaults limit to 80 when omitted', async () => {
      resolveAudienceMock.mockResolvedValueOnce([]);
      vi.mocked(insertBroadcastSnapshot).mockResolvedValueOnce(makeSnapshot({ recipients: [] }));

      await request(
        'POST',
        '/admin/broadcast',
        { template: 'release-announcement', audience: 'beta:active', dryRun: true },
        ADMIN_KEY
      );

      expect(resolveAudienceMock).toHaveBeenCalledWith(
        expect.anything(),
        'beta:active',
        expect.objectContaining({ limit: 80 })
      );
    });
  });

  describe('real-send', () => {
    const snapshotRecipients = [
      { email: 'alice@example.com', name: 'Alice' },
      { email: 'bob@example.com', name: null },
    ];

    function makeSnapshot(overrides: Partial<Record<string, unknown>> = {}) {
      return {
        token: 'snap-bc-abc',
        template: 'release-announcement',
        template_props: { version: 'v0.7.0-beta', theme: 'Snapshotted theme' },
        audience_key: 'beta:active',
        audience_params: {},
        recipients: snapshotRecipients,
        created_by_actor: 'shared-key@anvil',
        created_at: '2026-05-24T09:00:00Z',
        expires_at: '2026-05-24T09:10:00Z',
        consumed_at: null,
        ...overrides,
      };
    }

    it('accepts a preview-token-only real-send (no request template/audience)', async () => {
      // EMAIL-010 / #1926: the snapshot is source of truth on real-send, so
      // an operator may send ONLY {dryRun: false, previewToken}. The shared
      // request schema must not reject this for a missing template/audience.
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      const res = await request(
        'POST',
        '/admin/broadcast',
        { dryRun: false, previewToken: 'snap-bc-abc' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      // Response reflects the consumed snapshot, not the (absent) request body.
      expect(body.template).toBe('release-announcement');
      expect(body.audience).toBe('beta:active');
      expect(body.sent).toBe(2);
      expect(body.failed).toBe(0);
      expect(vi.mocked(consumeBroadcastSnapshot)).toHaveBeenCalledTimes(1);
    });

    it('ignores request-time template/audience/templateProps that contradict the consumed snapshot', async () => {
      // Anti-bait-and-switch: even when the operator re-supplies request-time
      // fields that DISAGREE with the snapshot, the consumed snapshot wins for
      // template, audience, and templateProps.
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({
          template: 'release-announcement',
          template_props: { version: 'v0.7.0-beta', theme: 'Snapshotted theme' },
          audience_key: 'beta:active',
          audience_params: {},
          recipients: [{ email: 'alice@example.com', name: 'Alice' }],
        })
      );
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          dryRun: false,
          previewToken: 'snap-bc-abc',
          // All three contradict the snapshot — must be ignored.
          template: 'waitlist-migration',
          audience: 'waitlist:source',
          audienceParams: { source: 'import' },
          templateProps: { version: 'v9.9.9', theme: 'Malicious theme' },
        },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.template).toBe('release-announcement');
      expect(body.audience).toBe('beta:active');

      // Re-resolve used the snapshot's audience key, not the request's.
      const audienceCall = resolveAudienceMock.mock.calls.at(-1);
      expect(audienceCall?.[1]).toBe('beta:active');

      // Sender received the snapshotted props, not the request's malicious ones.
      expect(vi.mocked(sendReleaseAnnouncement)).toHaveBeenCalledWith('alice@example.com', {
        version: 'v0.7.0-beta',
        theme: 'Snapshotted theme',
      });
    });

    it('returns 400 preview_token_required when token is missing', async () => {
      const res = await request(
        'POST',
        '/admin/broadcast',
        { template: 'release-announcement', audience: 'beta:active', dryRun: false },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('preview_token_required');
      expect(vi.mocked(consumeBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('returns 410 preview_token_missing when the token is unknown', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
      vi.mocked(findBroadcastSnapshot).mockResolvedValue(null);

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'ghost-token',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(410);
      expect((await res.json()).code).toBe('preview_token_missing');
    });

    it('returns 410 preview_token_consumed when the token was already used', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
      vi.mocked(findBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ consumed_at: '2026-05-24T09:05:00Z' })
      );

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(410);
      expect((await res.json()).code).toBe('preview_token_consumed');
    });

    it('returns 410 preview_token_expired when the token is past TTL', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
      vi.mocked(findBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ expires_at: '2026-05-24T08:00:00Z' })
      );

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(410);
      expect((await res.json()).code).toBe('preview_token_expired');
    });

    it('returns 409 cohort_drift when re-resolved recipients differ', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'charlie@example.com', name: 'Charlie', user_id: 'u-3' },
        // bob removed; charlie added
      ]);

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(409);
      const body = await res.json();
      expect(body.code).toBe('cohort_drift');
      expect(body.added).toEqual(['charlie@example.com']);
      expect(body.removed).toEqual(['bob@example.com']);
    });

    it('returns 409 cohort_drift when the cohort GREW beyond snapshot size (freshLimit+1)', async () => {
      // Snapshot has 2 recipients (alice, bob). Cohort now has 3 (a new
      // dave joined between dry-run and real-send). freshLimit+1=3 so
      // resolveAudience returns 3 rows, computeCohortDrift flags dave
      // as `added`. Without the +1, the resolver would return just the
      // snapshot's 2 rows and drift would not be detected.
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
        { email: 'dave@example.com', name: 'Dave', user_id: 'u-4' },
      ]);

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(409);
      const body = await res.json();
      expect(body.code).toBe('cohort_drift');
      expect(body.added).toEqual(['dave@example.com']);
      expect(body.removed).toEqual([]);

      // Confirm the re-resolve used freshLimit + 1 = 3, not the
      // snapshot size 2.
      const audienceCall = resolveAudienceMock.mock.calls.at(-1);
      expect(audienceCall?.[2]).toMatchObject({ limit: 3 });
    });

    it('returns 400 template_kind_not_broadcastable when snapshot audience_key is unknown', async () => {
      // Simulate a snapshot whose audience_key was removed from
      // AUDIENCE_KEYS between snapshot and consume (or written
      // directly to the DB). Without the guard, executeBroadcastFromSnapshot's
      // switch returns undefined → TypeError → 500.
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ audience_key: 'beta:ghost-audience' })
      );

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
      expect((await res.json()).code).toBe('template_kind_not_broadcastable');
    });

    it('writes broadcast.email.dispatch_started before the loop, broadcast.email.sent after', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ recipients: [{ email: 'alice@example.com', name: 'Alice' }] })
      );
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const actions = vi.mocked(insertAuditLog).mock.calls.map((c) => c[1]);
      expect(actions).toEqual(['broadcast.email.dispatch_started', 'broadcast.email.sent']);
    });

    it('writes broadcast.email.blocked with reason=cohort_drift on drift', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        // bob removed
      ]);

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const auditCalls = vi.mocked(insertAuditLog).mock.calls;
      const actions = auditCalls.map((c) => c[1]);
      expect(actions).toContain('broadcast.email.dispatch_started');
      expect(actions).toContain('broadcast.email.blocked');
      expect(actions).not.toContain('broadcast.email.sent');
      const blocked = auditCalls.find((c) => c[1] === 'broadcast.email.blocked');
      expect(blocked?.[3]).toMatchObject({ reason: 'cohort_drift' });
    });

    it('writes broadcast.email.blocked with reason=invalid_template when registry mutated', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ audience_key: 'beta:ghost-audience' })
      );

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const auditCalls = vi.mocked(insertAuditLog).mock.calls;
      const blocked = auditCalls.find((c) => c[1] === 'broadcast.email.blocked');
      expect(blocked?.[3]).toMatchObject({ reason: 'invalid_template' });
    });

    it('keeps the batch going when a sender throws (per-recipient try/catch)', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
      ]);
      vi.mocked(sendReleaseAnnouncement)
        .mockResolvedValueOnce({ sent: true })
        .mockRejectedValueOnce(new Error('Resend SDK crashed'));
      vi.spyOn(console, 'error').mockImplementation(() => undefined);

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.sent).toBe(1);
      expect(body.failed).toBe(1);
      expect(body.results[1]).toMatchObject({
        email: 'bob@example.com',
        sent: false,
        error: 'Resend SDK crashed',
      });
    });

    it('on clean send, iterates snapshot rows and returns sent/failed counts', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.template).toBe('release-announcement');
      expect(body.audience).toBe('beta:active');
      expect(body.total).toBe(2);
      expect(body.sent).toBe(2);
      expect(body.failed).toBe(0);
      expect(vi.mocked(sendReleaseAnnouncement)).toHaveBeenCalledTimes(2);
    });

    it('passes snapshot templateProps (not request templateProps) to the sender — bait-and-switch defence', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({
          template_props: { version: 'v0.7.0-beta', theme: 'Snapshotted theme' },
          recipients: [{ email: 'alice@example.com', name: 'Alice' }],
        })
      );
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
          // Operator tries to change the props after preview — must be ignored.
          templateProps: { version: 'v9.9.9', theme: 'Malicious theme' },
        },
        ADMIN_KEY
      );

      expect(vi.mocked(sendReleaseAnnouncement)).toHaveBeenCalledWith('alice@example.com', {
        version: 'v0.7.0-beta',
        theme: 'Snapshotted theme',
      });
    });

    it('uses snapshot audience (not request audience) on re-resolve — bait-and-switch defence', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({ audience_key: 'beta:active-recent', audience_params: {} })
      );
      resolveAudienceMock.mockResolvedValueOnce(
        snapshotRecipients.map((r) => ({ ...r, user_id: 'u-x' }))
      );
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          // Request says beta:active, but snapshot says beta:active-recent —
          // the re-resolve must use the snapshot's key.
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const audienceCall = resolveAudienceMock.mock.calls.at(-1);
      expect(audienceCall?.[1]).toBe('beta:active-recent');
    });

    it('reports failed sends per recipient without aborting the batch', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
      ]);
      vi.mocked(sendReleaseAnnouncement)
        .mockResolvedValueOnce({ sent: true })
        .mockResolvedValueOnce({ sent: false, code: 'provider_error', message: 'oops' });

      const res = await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const body = await res.json();
      expect(body.sent).toBe(1);
      expect(body.failed).toBe(1);
      expect(body.results).toEqual([
        { email: 'alice@example.com', sent: true, error: undefined },
        { email: 'bob@example.com', sent: false, error: 'oops' },
      ]);
    });

    it('writes a broadcast.email.sent audit log on success', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(makeSnapshot());
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: 'u-1' },
        { email: 'bob@example.com', name: null, user_id: 'u-2' },
      ]);
      vi.mocked(sendReleaseAnnouncement).mockResolvedValue({ sent: true });

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'release-announcement',
          audience: 'beta:active',
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      const auditCall = vi.mocked(insertAuditLog).mock.calls.at(-1);
      expect(auditCall?.[1]).toBe('broadcast.email.sent');
      const metadata = auditCall?.[3] as { previewTokenHash: string; [k: string]: unknown };
      expect(metadata).toMatchObject({
        template: 'release-announcement',
        audience: 'beta:active',
        sent: 2,
        failed: 0,
      });
      // Token is hashed in metadata so audit_log doesn't leak the bearer.
      // The hashToken mock returns 'mocked-hash'; the assertion that
      // matters is that the raw 'snap-bc-abc' is NOT in the metadata.
      expect(metadata.previewTokenHash).toBe('mocked-hash');
      expect(metadata.previewTokenHash).not.toBe('snap-bc-abc');
      expect(metadata).not.toHaveProperty('previewToken');
    });

    it('routes waitlist-migration through the registry sender', async () => {
      vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
        makeSnapshot({
          template: 'waitlist-migration',
          template_props: {},
          audience_key: 'waitlist:source',
          audience_params: { source: 'import' },
          recipients: [{ email: 'alice@example.com', name: 'Alice' }],
        })
      );
      resolveAudienceMock.mockResolvedValueOnce([
        { email: 'alice@example.com', name: 'Alice', user_id: null },
      ]);
      vi.mocked(sendWaitlistMigration).mockResolvedValue({ sent: true });

      await request(
        'POST',
        '/admin/broadcast',
        {
          template: 'waitlist-migration',
          audience: 'waitlist:source',
          audienceParams: { source: 'import' },
          dryRun: false,
          previewToken: 'snap-bc-abc',
        },
        ADMIN_KEY
      );

      expect(vi.mocked(sendWaitlistMigration)).toHaveBeenCalledWith('alice@example.com', 'Alice');
    });
  });
});
