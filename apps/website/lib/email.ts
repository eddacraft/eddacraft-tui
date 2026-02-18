import { Resend } from 'resend';
import { WaitlistConfirmation } from './emails/waitlist-confirmation';

let client: Resend | null = null;

function getClient(): Resend | null {
  const apiKey = process.env.RESEND_API_KEY;
  if (!apiKey) return null;
  if (!client) {
    client = new Resend(apiKey);
  }
  return client;
}

const FROM_ADDRESS = 'Josh at EddaCraft <anvil@updates.eddacraft.ai>';
const REPLY_TO = 'josh@eddacraft.ai';

export async function sendWaitlistConfirmation(email: string): Promise<void> {
  const resend = getClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping confirmation email');
    return;
  }

  const subject = encodeURIComponent('Unsubscribe');
  const body = encodeURIComponent(`Please remove ${email} from the waitlist.`);
  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=${subject}&body=${body}`;

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
  }
}
