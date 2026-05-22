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

  it('ReleaseAnnouncement renders v0.7.0-beta defaults with recipient email', async () => {
    const html = await render(
      <ReleaseAnnouncement email="tester@example.com" unsubscribeMailto={UNSUBSCRIBE} />
    );

    expect(html).toContain('tester@example.com');
    expect(html).toContain('v0.7.0-beta');
    expect(html).toContain('Daemon-Working End-to-End Protection');
    expect(html).toContain('brew upgrade eddacraft/tap/anvil');
    expect(html).toContain('anvil auth login');
    expect(html).toContain('issues/1827');
    expect(html).toContain('Boring Week');
    expect(html).toContain(`href="${UNSUBSCRIBE}"`);
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });

  it('ReleaseAnnouncement is parameterisable for future releases (no Boring Week ask)', async () => {
    const html = await render(
      <ReleaseAnnouncement
        email="tester@example.com"
        version="v0.7.1-beta"
        theme="Patch — hook coexistence fixes"
        intro="A short patch release follows up on hook-coexistence friction reports."
        highlights={[
          {
            title: 'Lefthook re-entry fix',
            body: 'install/uninstall round-trip now byte-stable.',
          },
        ]}
        releaseUrl="https://github.com/eddacraft/anvil/releases/tag/v0.7.1-beta"
        upgradeCommands={[{ label: 'Homebrew', command: 'brew upgrade eddacraft/tap/anvil' }]}
        firstInvocationNote={undefined}
        knownGaps={undefined}
        boringWeekAsk={undefined}
        migrationUrl={undefined}
        unsubscribeMailto={UNSUBSCRIBE}
      />
    );

    expect(html).toContain('v0.7.1-beta');
    expect(html).toContain('Patch — hook coexistence fixes');
    expect(html).not.toContain('Boring Week');
    expect(html).not.toContain('authRequired');
    expect(html).not.toContain('issues/1827');
    expect(html).not.toContain('undefined');
    expect(html).toMatch(/^<!DOCTYPE html/i);
  });
});
