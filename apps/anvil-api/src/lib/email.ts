import { Resend } from 'resend';
import { OtpCode, WaitlistConfirmation } from '@eddacraft/transactional';

let client: Resend | null = null;

function getResendClient(): Resend | null {
  const apiKey = process.env.RESEND_API_KEY;
  if (!apiKey) return null;
  if (!client) {
    client = new Resend(apiKey);
  }
  return client;
}

const FROM_ADDRESS = 'Josh at EddaCraft <anvil@updates.eddacraft.ai>';
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

—
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
