import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const senderMocks = vi.hoisted(() => ({
  sendWaitlistMigration: vi.fn(),
  sendReleaseAnnouncement: vi.fn(),
  sendBetaInvite: vi.fn(),
  sendOtpCode: vi.fn(),
  sendWaitlistConfirmation: vi.fn(),
}));

vi.mock('../email.js', () => senderMocks);

afterEach(() => {
  vi.restoreAllMocks();
});

import {
  EMAIL_REGISTRY,
  TEMPLATE_KEYS,
  type BroadcastTemplateKey,
  type TemplateKey,
  type TransactionalTemplateKey,
  isBroadcastTemplate,
} from '../email-registry.js';

const BROADCAST_KEYS: BroadcastTemplateKey[] = ['release-announcement', 'waitlist-migration'];
const TRANSACTIONAL_KEYS: TransactionalTemplateKey[] = [
  'beta-invite',
  'otp-code',
  'waitlist-confirmation',
];

describe('TEMPLATE_KEYS', () => {
  it('enumerates exactly the five v1 templates', () => {
    expect([...TEMPLATE_KEYS].sort()).toEqual(
      [
        'release-announcement',
        'waitlist-migration',
        'beta-invite',
        'otp-code',
        'waitlist-confirmation',
      ].sort()
    );
  });
});

describe('EMAIL_REGISTRY kind discrimination', () => {
  it.each(BROADCAST_KEYS)('%s is classified as broadcast', (key) => {
    expect(EMAIL_REGISTRY[key].kind).toBe('broadcast');
  });

  it.each(TRANSACTIONAL_KEYS)('%s is classified as transactional', (key) => {
    expect(EMAIL_REGISTRY[key].kind).toBe('transactional');
  });

  it.each(BROADCAST_KEYS)('isBroadcastTemplate returns true for %s', (key) => {
    expect(isBroadcastTemplate(key)).toBe(true);
  });

  it.each(TRANSACTIONAL_KEYS)('isBroadcastTemplate returns false for %s', (key) => {
    expect(isBroadcastTemplate(key)).toBe(false);
  });
});

describe('propsSchema — release-announcement', () => {
  const schema = EMAIL_REGISTRY['release-announcement'].propsSchema;

  it('accepts an empty payload (relies on v0.7.0 defaults baked into the template)', () => {
    expect(schema.safeParse({}).success).toBe(true);
  });

  it('accepts a fully populated payload', () => {
    const result = schema.safeParse({
      version: 'v0.8.0-beta',
      theme: 'Test theme',
      intro: 'Test intro',
      highlights: [{ title: 'h', body: 'b' }],
      releaseUrl: 'https://example.com',
      upgradeCommands: [{ label: 'brew', command: 'brew upgrade x' }],
      firstInvocationNote: { state: 's', recovery: 'r', rationale: 'why' },
      migrationUrl: 'https://example.com/migrate',
      knownGaps: [{ title: 't', body: 'b', trackingUrl: 'https://example.com/issue/1' }],
      boringWeekAsk: { durationLabel: 'a week', participantCount: 'three', replyInstruction: 'i' },
      feedbackEmail: 'feedback@example.com',
    });
    expect(result.success).toBe(true);
  });

  it('rejects unknown top-level keys', () => {
    const result = schema.safeParse({ unknownField: 'oops' });
    expect(result.success).toBe(false);
  });

  it('rejects releaseUrl that is not https://', () => {
    expect(schema.safeParse({ releaseUrl: 'http://example.com' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'javascript:alert(1)' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'data:text/html,<script>' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'https://example.com/v0.7.0' }).success).toBe(true);
  });

  it('rejects releaseUrl with userinfo (https://user:pass@evil.com)', () => {
    expect(schema.safeParse({ releaseUrl: 'https://user:pass@evil.com' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'https://attacker@evil.com' }).success).toBe(false);
  });

  it('rejects releaseUrl with leading/trailing whitespace', () => {
    expect(schema.safeParse({ releaseUrl: ' https://example.com' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'https://example.com\n' }).success).toBe(false);
  });

  it('rejects releaseUrl with embedded control characters', () => {
    expect(schema.safeParse({ releaseUrl: 'https://example.com\nattack' }).success).toBe(false);
    expect(schema.safeParse({ releaseUrl: 'https://example.com\tattack' }).success).toBe(false);
  });

  it('caps highlights at 20 elements', () => {
    const tooMany = Array.from({ length: 21 }, (_, i) => ({ title: `t${i}`, body: 'b' }));
    expect(schema.safeParse({ highlights: tooMany }).success).toBe(false);
    const justRight = Array.from({ length: 20 }, (_, i) => ({ title: `t${i}`, body: 'b' }));
    expect(schema.safeParse({ highlights: justRight }).success).toBe(true);
  });

  it('caps upgradeCommands and knownGaps at 20 elements', () => {
    const tooMany = Array.from({ length: 21 }, (_, i) => ({ label: `l${i}`, command: 'c' }));
    expect(schema.safeParse({ upgradeCommands: tooMany }).success).toBe(false);
    const tooManyGaps = Array.from({ length: 21 }, (_, i) => ({ title: `t${i}`, body: 'b' }));
    expect(schema.safeParse({ knownGaps: tooManyGaps }).success).toBe(false);
  });

  it('rejects migrationUrl that is not https://', () => {
    expect(schema.safeParse({ migrationUrl: 'http://example.com' }).success).toBe(false);
    expect(schema.safeParse({ migrationUrl: 'https://example.com/migrate' }).success).toBe(true);
  });

  it('rejects knownGaps[].trackingUrl that is not https://', () => {
    expect(
      schema.safeParse({ knownGaps: [{ title: 't', body: 'b', trackingUrl: 'http://x.com' }] })
        .success
    ).toBe(false);
    expect(
      schema.safeParse({ knownGaps: [{ title: 't', body: 'b', trackingUrl: 'https://x.com' }] })
        .success
    ).toBe(true);
  });

  it('rejects feedbackEmail that is not a valid email', () => {
    expect(schema.safeParse({ feedbackEmail: 'not-an-email' }).success).toBe(false);
    expect(schema.safeParse({ feedbackEmail: 'a@b.co' }).success).toBe(true);
  });

  it('rejects email — it must come from the recipient row, not operator props', () => {
    const result = schema.safeParse({ email: 'a@x.com' });
    expect(result.success).toBe(false);
  });

  it('rejects unsubscribeMailto — it is computed at send time from the recipient email', () => {
    const result = schema.safeParse({ unsubscribeMailto: 'mailto:x@y.com' });
    expect(result.success).toBe(false);
  });
});

describe('propsSchema — waitlist-migration', () => {
  it('is empty: operator supplies no props; name comes from the row', () => {
    const schema = EMAIL_REGISTRY['waitlist-migration'].propsSchema;
    expect(schema.safeParse({}).success).toBe(true);
    expect(schema.safeParse({ name: 'Alice' }).success).toBe(false);
  });
});

describe('propsSchema — beta-invite', () => {
  const schema = EMAIL_REGISTRY['beta-invite'].propsSchema;

  it('requires userCode and activateUrl', () => {
    expect(schema.safeParse({}).success).toBe(false);
    expect(schema.safeParse({ userCode: 'ANVIL-12345678' }).success).toBe(false);
    expect(
      schema.safeParse({ userCode: 'ANVIL-12345678', activateUrl: 'https://example.com' }).success
    ).toBe(true);
  });
});

describe('propsSchema — otp-code', () => {
  const schema = EMAIL_REGISTRY['otp-code'].propsSchema;

  it('requires code', () => {
    expect(schema.safeParse({}).success).toBe(false);
    expect(schema.safeParse({ code: '123456' }).success).toBe(true);
  });
});

describe('propsSchema — waitlist-confirmation', () => {
  it('is empty: operator supplies no props', () => {
    const schema = EMAIL_REGISTRY['waitlist-confirmation'].propsSchema;
    expect(schema.safeParse({}).success).toBe(true);
  });
});

describe('broadcast senders', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('release-announcement sender forwards row email + props to sendReleaseAnnouncement', async () => {
    senderMocks.sendReleaseAnnouncement.mockResolvedValueOnce({ sent: true });
    const entry = EMAIL_REGISTRY['release-announcement'];
    if (entry.kind !== 'broadcast') throw new Error('expected broadcast kind');

    await entry.sender(
      { email: 'a@x.com', name: 'Alice', user_id: 'u-1' },
      { version: 'v0.8.0-beta', theme: 'Test' }
    );

    expect(senderMocks.sendReleaseAnnouncement).toHaveBeenCalledWith('a@x.com', {
      version: 'v0.8.0-beta',
      theme: 'Test',
    });
  });

  it('release-announcement sender does not pull row.name into props (template has no name field)', async () => {
    senderMocks.sendReleaseAnnouncement.mockResolvedValueOnce({ sent: true });
    const entry = EMAIL_REGISTRY['release-announcement'];
    if (entry.kind !== 'broadcast') throw new Error('expected broadcast kind');

    await entry.sender({ email: 'a@x.com', name: 'Alice', user_id: 'u-1' }, {});

    const callProps = senderMocks.sendReleaseAnnouncement.mock.calls.at(-1)?.[1] as Record<
      string,
      unknown
    >;
    expect(callProps).not.toHaveProperty('name');
  });

  it('waitlist-migration sender forwards row email + name to sendWaitlistMigration', async () => {
    senderMocks.sendWaitlistMigration.mockResolvedValueOnce({ sent: true });
    const entry = EMAIL_REGISTRY['waitlist-migration'];
    if (entry.kind !== 'broadcast') throw new Error('expected broadcast kind');

    await entry.sender({ email: 'alice@example.com', name: 'Alice', user_id: null }, {});

    expect(senderMocks.sendWaitlistMigration).toHaveBeenCalledWith('alice@example.com', 'Alice');
  });

  it('waitlist-migration sender passes undefined when name is null', async () => {
    senderMocks.sendWaitlistMigration.mockResolvedValueOnce({ sent: true });
    const entry = EMAIL_REGISTRY['waitlist-migration'];
    if (entry.kind !== 'broadcast') throw new Error('expected broadcast kind');

    await entry.sender({ email: 'bob@example.com', name: null, user_id: null }, {});

    expect(senderMocks.sendWaitlistMigration).toHaveBeenCalledWith('bob@example.com', undefined);
  });
});

describe('TypeScript discriminated union', () => {
  it('transactional entries have no sender field at the type level', () => {
    // This is a compile-time guarantee; the runtime check just confirms
    // the registry shape matches the documented contract.
    const entry = EMAIL_REGISTRY['otp-code'];
    expect(entry.kind).toBe('transactional');
    expect('sender' in entry).toBe(false);
  });

  it('broadcast entries always have a callable sender', () => {
    for (const key of BROADCAST_KEYS) {
      const entry = EMAIL_REGISTRY[key];
      expect(entry.kind).toBe('broadcast');
      if (entry.kind === 'broadcast') {
        expect(typeof entry.sender).toBe('function');
      }
    }
  });
});

describe('TemplateKey type exhaustiveness', () => {
  it('TEMPLATE_KEYS covers every key in EMAIL_REGISTRY', () => {
    const registryKeys = Object.keys(EMAIL_REGISTRY).sort() as TemplateKey[];
    expect([...TEMPLATE_KEYS].sort()).toEqual(registryKeys);
  });
});
