import { Resend } from 'resend';
import {
  BetaInvite,
  OtpCode,
  ReleaseAnnouncement,
  V070_DEFAULTS,
  WaitlistConfirmation,
  WaitlistMigration,
} from '@eddacraft/transactional';

export type ReleaseAnnouncementSendProps = Partial<{
  version: string;
  theme: string;
  intro: string;
  highlights: Array<{ title: string; body: string }>;
  releaseUrl: string;
  upgradeCommands: Array<{ label: string; command: string }>;
  firstInvocationNote: { state: string; recovery: string; rationale: string };
  migrationUrl: string;
  knownGaps: Array<{ title: string; body: string; trackingUrl?: string }>;
  boringWeekAsk: { durationLabel: string; participantCount: string; replyInstruction: string };
  feedbackEmail: string;
}>;

let client: Resend | null = null;

function getResendClient(): Resend | null {
  const apiKey = process.env.RESEND_API_KEY;
  if (!apiKey) return null;
  if (!client) {
    client = new Resend(apiKey);
  }
  return client;
}

const FROM_ADDRESS = 'Josh at eddacraft <anvil@updates.eddacraft.ai>';
const REPLY_TO = 'josh@eddacraft.ai';

export interface EmailDeliveryResult {
  sent: boolean;
  code?: string;
  message?: string;
}

export async function sendWaitlistConfirmation(email: string): Promise<EmailDeliveryResult> {
  const resend = getResendClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping confirmation email');
    return { sent: false, code: 'resend_not_configured', message: 'Resend is not configured' };
  }

  const subject = encodeURIComponent('Unsubscribe');
  const body = encodeURIComponent(`Please remove ${email} from the waitlist.`);
  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=${subject}&body=${body}`;

  try {
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      replyTo: REPLY_TO,
      to: email,
      subject: "You're on the Anvil waitlist",
      headers: {
        'List-Unsubscribe': `<${unsubscribeMailto}>`,
      },
      react: WaitlistConfirmation({ email, unsubscribeMailto }),
      text: `$ anvil :: waitlist confirm

[ OK ] Access request received

Your email ${email} has been added to the Anvil waitlist.

We're onboarding engineering teams in controlled cohorts. You'll hear from us when your slot opens.

If you have any questions or feedback, just reply to this email — I personally respond to each one.

— Josh
Founder, eddacraft

anvil :: eddacraft.ai

To unsubscribe, reply with "unsubscribe" or visit: ${unsubscribeMailto}`,
      tags: [{ name: 'category', value: 'waitlist-confirmation' }],
    });

    if (error) {
      console.error('Failed to send waitlist confirmation email:', error.message);
      return { sent: false, code: 'provider_error', message: error.message };
    }

    return { sent: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Unexpected waitlist email delivery error:', message);
    return { sent: false, code: 'unexpected_error', message };
  }
}

export async function sendWaitlistAdminNotification(
  signupEmail: string,
  isNewSignup: boolean,
  emailSent: boolean
): Promise<void> {
  const resend = getResendClient();
  const adminEmail = process.env.WAITLIST_ADMIN_EMAIL;
  if (!resend || !adminEmail) {
    console.warn('Admin notification skipped — missing env', {
      hasResend: !!resend,
      hasAdminEmail: !!adminEmail,
    });
    return;
  }

  try {
    const status = !isNewSignup ? 'skipped (returning signup)' : emailSent ? 'sent' : 'FAILED';
    const label = isNewSignup ? 'New signup' : 'Returning signup';
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      to: adminEmail,
      subject: `[Anvil Waitlist] ${label}: ${signupEmail}`,
      text: `${label} on the Anvil waitlist.\n\nEmail: ${signupEmail}\nConfirmation: ${status}\nTime: ${new Date().toISOString()}`,
      tags: [{ name: 'category', value: 'waitlist-admin-notification' }],
    });
    if (error) {
      console.error('Failed to send admin notification:', error.message);
    }
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    console.error('Failed to send admin notification:', message);
  }
}

export async function sendOtpCode(email: string, code: string): Promise<EmailDeliveryResult> {
  const resend = getResendClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping OTP email');
    return { sent: false, code: 'resend_not_configured', message: 'Resend is not configured' };
  }

  const unsubscribeMailto = 'mailto:anvil@updates.eddacraft.ai';

  try {
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      replyTo: REPLY_TO,
      to: email,
      subject: 'Your Anvil verification code',
      react: OtpCode({ code, unsubscribeMailto }),
      text: `Your Anvil verification code is: ${code}\n\nThis code expires in 10 minutes.\nIf you didn't request this, you can safely ignore it.\n\n—\nanvil :: eddacraft.ai`,
      tags: [{ name: 'category', value: 'otp-code' }],
    });

    if (error) {
      console.error('Failed to send OTP email:', error.message);
      return { sent: false, code: 'provider_error', message: error.message };
    }

    return { sent: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Unexpected OTP email delivery error:', message);
    return { sent: false, code: 'unexpected_error', message };
  }
}

export async function sendWaitlistMigration(
  email: string,
  name?: string
): Promise<EmailDeliveryResult> {
  const resend = getResendClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping migration email');
    return { sent: false, code: 'resend_not_configured', message: 'Resend is not configured' };
  }

  const subject = encodeURIComponent('Unsubscribe');
  const body = encodeURIComponent(`Please remove ${email} from communications.`);
  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=${subject}&body=${body}`;

  try {
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      replyTo: REPLY_TO,
      to: email,
      subject: "Anvil has a new home — and you're on the early access waitlist",
      headers: {
        'List-Unsubscribe': `<${unsubscribeMailto}>`,
      },
      react: WaitlistMigration({ email, name, unsubscribeMailto }),
      text: `$ anvil :: status update

[ INFO ] Platform update

${name ? `${name}, you` : 'You'} signed up for early notifications on Anvil. A lot has changed since then.

What's new:

New website — We've rebuilt eddacraft.ai from the ground up. It's faster, cleaner, and reflects where the product is heading.

Documentation — Full docs are now live at docs.eddacraft.ai. Architecture guides, CLI reference, and getting started walkthroughs are all there.

Beta waitlist — Your email ${email} has been moved to the formal early access waitlist. You don't need to sign up again. When your cohort opens, you'll receive an invite with activation instructions.

We're onboarding engineering teams in controlled cohorts to keep quality high. Capacity is limited — early signups like yours are prioritised.

Questions or feedback? Just reply to this email — I personally read and respond to every one.

— Josh
Founder, eddacraft

anvil :: eddacraft.ai

To unsubscribe, reply with "unsubscribe" or visit: ${unsubscribeMailto}`,
      tags: [{ name: 'category', value: 'waitlist-migration' }],
    });

    if (error) {
      console.error('Failed to send migration email:', error.message);
      return { sent: false, code: 'provider_error', message: error.message };
    }

    return { sent: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Unexpected migration email delivery error:', message);
    return { sent: false, code: 'unexpected_error', message };
  }
}

export async function sendBetaInvite(
  email: string,
  userCode: string,
  activateUrl: string
): Promise<EmailDeliveryResult> {
  const resend = getResendClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping invite email');
    return { sent: false, code: 'resend_not_configured', message: 'Resend is not configured' };
  }

  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=${encodeURIComponent('Unsubscribe')}&body=${encodeURIComponent(`Please remove ${email} from beta communications.`)}`;

  try {
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      replyTo: REPLY_TO,
      to: email,
      subject: "You're in — Anvil early access",
      react: BetaInvite({ email, userCode, activateUrl, unsubscribeMailto }),
      text: `You're in — Anvil early access\n\nYour email ${email} has been approved for Anvil early access.\n\nActivate in your browser:\n${activateUrl}\n\nOr run in your terminal:\n$ anvil auth login\n\nYour activation code: ${userCode}\nThis code expires in 48 hours.\n\n—\nanvil :: eddacraft.ai`,
      tags: [{ name: 'category', value: 'beta-invite' }],
    });

    if (error) {
      console.error('Failed to send beta invite email:', error.message);
      return { sent: false, code: 'provider_error', message: error.message };
    }
    return { sent: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Unexpected beta invite email delivery error:', message);
    return { sent: false, code: 'unexpected_error', message };
  }
}

export async function sendReleaseAnnouncement(
  email: string,
  props: ReleaseAnnouncementSendProps
): Promise<EmailDeliveryResult> {
  const resend = getResendClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping release-announcement email');
    return { sent: false, code: 'resend_not_configured', message: 'Resend is not configured' };
  }

  const subject = encodeURIComponent('Unsubscribe');
  const body = encodeURIComponent(`Please remove ${email} from release announcements.`);
  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=${subject}&body=${body}`;

  // Subject derivation: fall back to V070_DEFAULTS per-field rather than
  // all-or-nothing. A partial supply (e.g. `version: 'v0.8.0-beta'` with no
  // theme) previously produced `Anvil v0.8.0-beta — ` with an empty theme
  // half. Per-field fallback keeps the subject readable when the operator
  // overrides only one identifier.
  const version = props.version ?? V070_DEFAULTS.version;
  const theme = props.theme ?? V070_DEFAULTS.theme;
  const emailSubject = `Anvil ${version} — ${theme}`;

  try {
    const { error } = await resend.emails.send({
      from: FROM_ADDRESS,
      replyTo: REPLY_TO,
      to: email,
      subject: emailSubject,
      headers: {
        'List-Unsubscribe': `<${unsubscribeMailto}>`,
      },
      // Spread props first, then override with sender-controlled fields so an
      // operator cannot smuggle a different email or unsubscribeMailto through
      // templateProps. The email-registry strict schema rejects them at the
      // /admin/broadcast boundary; this is belt-and-braces at the sender.
      react: ReleaseAnnouncement({ ...props, email, unsubscribeMailto }),
      text: `Anvil ${version} — ${theme}

A new Anvil release is live. Full notes and upgrade commands are in the rendered email body; if you can't see HTML, the release notes URL is:

${props.releaseUrl ?? V070_DEFAULTS.releaseUrl}

To unsubscribe, reply with "unsubscribe" or visit: ${unsubscribeMailto}

— Josh
anvil :: eddacraft.ai`,
      tags: [{ name: 'category', value: 'release-announcement' }],
    });

    if (error) {
      console.error('Failed to send release announcement email:', error.message);
      return { sent: false, code: 'provider_error', message: error.message };
    }

    return { sent: true };
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    console.error('Unexpected release-announcement delivery error:', message);
    return { sent: false, code: 'unexpected_error', message };
  }
}
