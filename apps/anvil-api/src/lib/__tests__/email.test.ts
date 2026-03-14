import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const emailMocks = vi.hoisted(() => ({
  Resend: vi.fn(),
  send: vi.fn(),
  WaitlistConfirmation: vi.fn(),
}));

vi.mock('resend', () => ({
  Resend: emailMocks.Resend,
}));

vi.mock('@eddacraft/transactional', () => ({
  WaitlistConfirmation: emailMocks.WaitlistConfirmation,
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
