import { render } from 'react-email';
import { describe, expect, it } from 'vitest';

import { BetaInvite } from '../beta-invite.js';
import { OtpCode } from '../otp-code.js';
import { ReleaseAnnouncement } from '../release-announcement.js';
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

  it('BetaInvite includes the email and CLI login instructions, no activation code', async () => {
    const html = await render(
      <BetaInvite email="tester@example.com" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('tester@example.com');
    expect(html).toContain('curl -fsSL https://install.eddacraft.ai | sh');
    expect(html).toContain('irm https://install.eddacraft.ai/windows | iex');
    expect(html).toContain('href="https://docs.eddacraft.ai/anvil/beta-testing-guide"');
    expect(html).toContain('href="https://install.eddacraft.ai"');
    expect(html).toContain('href="https://docs.eddacraft.ai"');
    expect(html).toContain('anvil auth login');
    expect(html).toContain('--otp');
    expect(html).not.toContain('auth/activate');
    expect(html).not.toContain('activation code');
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

  it('ReleaseAnnouncement defaults to the current beta roundup', async () => {
    const html = await render(
      <ReleaseAnnouncement email="tester@example.com" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('v0.9.4-beta');
    expect(html).toContain('tester@example.com');
    expect(html).toContain('anvil auth login');
    expect(html).toContain('anvil update');
    expect(html).toContain('curl -fsSL https://install.eddacraft.ai | sh');
    expect(html).toContain('irm https://install.eddacraft.ai/windows | iex');
    expect(html).toContain('href="https://github.com/eddacraft/anvil/releases/tag/v0.9.4-beta"');
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });
});
