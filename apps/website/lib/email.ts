import { Resend } from 'resend';

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

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export async function sendWaitlistConfirmation(email: string): Promise<void> {
  const resend = getClient();
  if (!resend) {
    console.warn('RESEND_API_KEY not configured — skipping confirmation email');
    return;
  }

  const safeEmail = escapeHtml(email);
  const unsubscribeMailto = `mailto:anvil@updates.eddacraft.ai?subject=Unsubscribe&body=Please remove ${email} from the waitlist.`;

  const { error } = await resend.emails.send({
    from: FROM_ADDRESS,
    reply_to: REPLY_TO,
    to: email,
    subject: "You're on the Anvil waitlist",
    headers: {
      'List-Unsubscribe': `<${unsubscribeMailto}>`,
    },
    text: `$ anvil :: waitlist confirm

[ OK ] Access request received

Your email ${email} has been added to the Anvil waitlist.

We're onboarding engineering teams in controlled cohorts. You'll hear from us when your slot opens.

If you have any questions or feedback, just reply to this email — I personally respond to each one.

—
anvil :: eddacraft.ai

To unsubscribe, reply with "unsubscribe" or visit: ${unsubscribeMailto}`,
    html: `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="margin:0;padding:0;background-color:#0a0a0a;font-family:'Courier New',Courier,monospace;color:#d4d4d4;">
  <table width="100%" cellpadding="0" cellspacing="0" style="background-color:#0a0a0a;padding:40px 20px;">
    <tr>
      <td align="center">
        <table width="560" cellpadding="0" cellspacing="0" style="max-width:560px;width:100%;">
          <tr>
            <td style="padding-bottom:24px;border-bottom:1px solid #262626;">
              <span style="font-size:14px;color:#737373;">$ </span>
              <span style="font-size:14px;color:#d4d4d4;font-weight:bold;">anvil</span>
              <span style="font-size:14px;color:#737373;"> :: waitlist confirm</span>
            </td>
          </tr>
          <tr>
            <td style="padding:32px 0;">
              <p style="margin:0 0 16px;font-size:14px;color:#22c55e;">[ OK ] Access request received</p>
              <p style="margin:0 0 24px;font-size:14px;color:#d4d4d4;">
                Your email <strong style="color:#f5f5f5;">${safeEmail}</strong> has been added to the Anvil waitlist.
              </p>
              <p style="margin:0 0 8px;font-size:13px;color:#a3a3a3;">
                We're onboarding engineering teams in controlled cohorts. You'll hear from us when your slot opens.
              </p>
              <p style="margin:16px 0 0;font-size:13px;color:#a3a3a3;">
                If you have any questions or feedback, just reply to this email — I personally respond to each one.
              </p>
            </td>
          </tr>
          <tr>
            <td style="padding-top:24px;border-top:1px solid #262626;">
              <p style="margin:0 0 8px;font-size:11px;color:#525252;">
                anvil :: eddacraft.ai
              </p>
              <p style="margin:0;font-size:10px;">
                <a href="${unsubscribeMailto}" style="color:#525252;text-decoration:underline;">unsubscribe</a>
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>`,
    tags: [{ name: 'category', value: 'waitlist-confirmation' }],
  });

  if (error) {
    console.error('Failed to send waitlist confirmation email:', error.message);
  }
}
