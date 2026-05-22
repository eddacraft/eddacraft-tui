import { render } from 'react-email';
import { describe, expect, it } from 'vitest';

import { BetaInvite } from '../beta-invite.js';
import { OtpCode } from '../otp-code.js';
import { WaitlistConfirmation } from '../waitlist-confirmation.js';
import { WaitlistMigration } from '../waitlist-migration.js';

const UNSUBSCRIBE = 'mailto:anvil@updates.eddacraft.ai?subject=Unsubscribe';

describe('transactional templates render to valid HTML', () => {
  it('OtpCode includes the code and unsubscribe link', async () => {
    const html = await render(<OtpCode code="847291" unsubscribeMailto={UNSUBSCRIBE} />);

    expect(html).toContain('847291');
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });

  it('BetaInvite includes the email, activation URL, and user code', async () => {
    const activateUrl = 'https://eddacraft.ai/auth/activate?code=ANVIL-7F3A';
    const html = await render(
      <BetaInvite
        email="tester@example.com"
        userCode="ANVIL-7F3A"
        activateUrl={activateUrl}
        unsubscribeMailto={UNSUBSCRIBE}
      />
    );

    expect(html).toContain('tester@example.com');
    expect(html).toContain('ANVIL-7F3A');
    expect(html).toContain(`href="${activateUrl}"`);
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });

  it('WaitlistConfirmation includes the email and unsubscribe link', async () => {
    const html = await render(
      <WaitlistConfirmation email="tester@example.com" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('tester@example.com');
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });

  it('WaitlistMigration includes the email and unsubscribe link (no name)', async () => {
    const html = await render(
      <WaitlistMigration email="tester@example.com" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('tester@example.com');
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });

  it('WaitlistMigration personalises when a name is provided', async () => {
    const html = await render(
      <WaitlistMigration email="tester@example.com" name="Josh" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('Josh');
    expect(html).toContain('tester@example.com');
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });
});
