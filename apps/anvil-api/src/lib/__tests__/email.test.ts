import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const emailMocks = vi.hoisted(() => ({
  Resend: vi.fn(),
  send: vi.fn(),
  WaitlistConfirmation: vi.fn(),
  ReleaseAnnouncement: vi.fn(),
  V070_DEFAULTS: {
    version: 'v0.7.0-beta',
    theme: 'Daemon-Working End-to-End Protection',
    intro: 'A new Anvil release is live.',
    highlights: [],
    releaseUrl: 'https://example.com/v0.7.0',
    upgradeCommands: [],
  },
}));

vi.mock('resend', () => ({
  Resend: emailMocks.Resend,
}));

vi.mock('@eddacraft/transactional', () => ({
  WaitlistConfirmation: emailMocks.WaitlistConfirmation,
  ReleaseAnnouncement: emailMocks.ReleaseAnnouncement,
  V070_DEFAULTS: emailMocks.V070_DEFAULTS,
}));

const originalResendApiKey = process.env['RESEND_API_KEY'];
type MockEmailClient = { emails: { send: typeof emailMocks.send } };

async function loadEmailModule() {
  vi.resetModules();
  return import('../email.js');
}

describe('sendWaitlistConfirmation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    emailMocks.Resend.mockImplementation(function ResendMock(this: MockEmailClient) {
      this.emails = {
        send: emailMocks.send,
      };
    });
    emailMocks.WaitlistConfirmation.mockImplementation(({ email, unsubscribeMailto }) => ({
      email,
      unsubscribeMailto,
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  afterAll(() => {
    if (originalResendApiKey === undefined) {
      delete process.env['RESEND_API_KEY'];
      return;
    }

    process.env['RESEND_API_KEY'] = originalResendApiKey;
  });

  it('returns resend_not_configured when the API key is missing', async () => {
    delete process.env['RESEND_API_KEY'];
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const { sendWaitlistConfirmation } = await loadEmailModule();

    const result = await sendWaitlistConfirmation('person@example.com');

    expect(result).toEqual({
      sent: false,
      code: 'resend_not_configured',
      message: 'Resend is not configured',
    });
    expect(emailMocks.Resend).not.toHaveBeenCalled();
    expect(warnSpy).toHaveBeenCalledWith(
      'RESEND_API_KEY not configured — skipping confirmation email'
    );
  });

  it('sends a waitlist confirmation email successfully', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendWaitlistConfirmation } = await loadEmailModule();

    const result = await sendWaitlistConfirmation('person@example.com');

    expect(result).toEqual({ sent: true });
    expect(emailMocks.Resend).toHaveBeenCalledWith('resend_test_key');
    expect(emailMocks.WaitlistConfirmation).toHaveBeenCalledWith({
      email: 'person@example.com',
      unsubscribeMailto: expect.stringContaining('mailto:anvil@updates.eddacraft.ai'),
    });
    expect(emailMocks.send).toHaveBeenCalledWith(
      expect.objectContaining({
        to: 'person@example.com',
        subject: "You're on the Anvil waitlist",
        replyTo: 'josh@eddacraft.ai',
        tags: [{ name: 'category', value: 'waitlist-confirmation' }],
        headers: {
          'List-Unsubscribe': expect.stringContaining(
            'mailto:anvil@updates.eddacraft.ai?subject=Unsubscribe'
          ),
        },
      })
    );
  });

  it('returns provider_error when Resend reports a delivery error', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: { message: 'Provider unavailable' } });
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const { sendWaitlistConfirmation } = await loadEmailModule();

    const result = await sendWaitlistConfirmation('person@example.com');

    expect(result).toEqual({
      sent: false,
      code: 'provider_error',
      message: 'Provider unavailable',
    });
    expect(errorSpy).toHaveBeenCalledWith(
      'Failed to send waitlist confirmation email:',
      'Provider unavailable'
    );
  });

  it('returns unexpected_error when the email client throws', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockRejectedValue(new Error('Network unavailable'));
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const { sendWaitlistConfirmation } = await loadEmailModule();

    const result = await sendWaitlistConfirmation('person@example.com');

    expect(result).toEqual({
      sent: false,
      code: 'unexpected_error',
      message: 'Network unavailable',
    });
    expect(errorSpy).toHaveBeenCalledWith(
      'Unexpected waitlist email delivery error:',
      'Network unavailable'
    );
  });
});

describe('sendReleaseAnnouncement', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    emailMocks.Resend.mockImplementation(function ResendMock(this: MockEmailClient) {
      this.emails = {
        send: emailMocks.send,
      };
    });
    emailMocks.ReleaseAnnouncement.mockImplementation((props) => ({
      type: 'ReleaseAnnouncement',
      props,
    }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  afterAll(() => {
    if (originalResendApiKey === undefined) {
      delete process.env['RESEND_API_KEY'];
      return;
    }

    process.env['RESEND_API_KEY'] = originalResendApiKey;
  });

  it('returns resend_not_configured when the API key is missing', async () => {
    delete process.env['RESEND_API_KEY'];
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    const { sendReleaseAnnouncement } = await loadEmailModule();

    const result = await sendReleaseAnnouncement('person@example.com', {});

    expect(result).toEqual({
      sent: false,
      code: 'resend_not_configured',
      message: 'Resend is not configured',
    });
    expect(warnSpy).toHaveBeenCalledWith(
      'RESEND_API_KEY not configured — skipping release-announcement email'
    );
  });

  it('uses V070_DEFAULTS for the subject when version + theme are both absent', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', {});

    expect(emailMocks.send).toHaveBeenCalledWith(
      expect.objectContaining({
        to: 'person@example.com',
        subject: 'Anvil v0.7.0-beta — Daemon-Working End-to-End Protection',
        replyTo: 'josh@eddacraft.ai',
        tags: [{ name: 'category', value: 'release-announcement' }],
        headers: {
          'List-Unsubscribe': expect.stringContaining(
            'mailto:anvil@updates.eddacraft.ai?subject=Unsubscribe'
          ),
        },
      })
    );
  });

  it('derives the subject from operator-supplied version + theme when both present', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', {
      version: 'v0.8.0-beta',
      theme: 'Boring Week Refinements',
    });

    expect(emailMocks.send).toHaveBeenCalledWith(
      expect.objectContaining({
        subject: 'Anvil v0.8.0-beta — Boring Week Refinements',
      })
    );
  });

  it('falls back to V070_DEFAULTS theme when only version is supplied', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', { version: 'v0.8.0-beta' });

    expect(emailMocks.send).toHaveBeenCalledWith(
      expect.objectContaining({
        subject: 'Anvil v0.8.0-beta — Daemon-Working End-to-End Protection',
      })
    );
  });

  it('falls back to V070_DEFAULTS version when only theme is supplied', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', { theme: 'Custom Theme' });

    expect(emailMocks.send).toHaveBeenCalledWith(
      expect.objectContaining({
        subject: 'Anvil v0.7.0-beta — Custom Theme',
      })
    );
  });

  it('passes through props to the ReleaseAnnouncement template alongside email + unsubscribeMailto', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', {
      version: 'v0.8.0-beta',
      theme: 'Test',
      intro: 'Test intro',
    });

    expect(emailMocks.ReleaseAnnouncement).toHaveBeenCalledWith(
      expect.objectContaining({
        email: 'person@example.com',
        version: 'v0.8.0-beta',
        theme: 'Test',
        intro: 'Test intro',
        unsubscribeMailto: expect.stringContaining('mailto:anvil@updates.eddacraft.ai'),
      })
    );
  });

  it('rejects an unsubscribeMailto smuggled through props (sender computes its own)', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: null });
    const { sendReleaseAnnouncement } = await loadEmailModule();

    await sendReleaseAnnouncement('person@example.com', {
      // @ts-expect-error — runtime guard test; props type excludes unsubscribeMailto
      unsubscribeMailto: 'mailto:attacker@evil.example',
    });

    const templateCall = emailMocks.ReleaseAnnouncement.mock.calls.at(-1)?.[0] as {
      unsubscribeMailto: string;
    };
    expect(templateCall.unsubscribeMailto).toContain('anvil@updates.eddacraft.ai');
    expect(templateCall.unsubscribeMailto).not.toContain('attacker@evil.example');
  });

  it('returns provider_error when Resend reports a delivery error', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockResolvedValue({ error: { message: 'Provider unavailable' } });
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const { sendReleaseAnnouncement } = await loadEmailModule();

    const result = await sendReleaseAnnouncement('person@example.com', {});

    expect(result).toEqual({
      sent: false,
      code: 'provider_error',
      message: 'Provider unavailable',
    });
    expect(errorSpy).toHaveBeenCalledWith(
      'Failed to send release announcement email:',
      'Provider unavailable'
    );
  });

  it('returns unexpected_error when the email client throws', async () => {
    process.env['RESEND_API_KEY'] = 'resend_test_key';
    emailMocks.send.mockRejectedValue(new Error('Network unavailable'));
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const { sendReleaseAnnouncement } = await loadEmailModule();

    const result = await sendReleaseAnnouncement('person@example.com', {});

    expect(result).toEqual({
      sent: false,
      code: 'unexpected_error',
      message: 'Network unavailable',
    });
    expect(errorSpy).toHaveBeenCalledWith(
      'Unexpected release-announcement delivery error:',
      'Network unavailable'
    );
  });
});
