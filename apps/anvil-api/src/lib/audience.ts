import { Resend } from 'resend';

let client: Resend | null = null;

function getResendClient(): Resend | null {
  const apiKey = process.env.RESEND_API_KEY;
  if (!apiKey) return null;
  if (!client) client = new Resend(apiKey);
  return client;
}

export async function addToWaitlistAudience(email: string): Promise<void> {
  const resend = getResendClient();
  const audienceId = process.env['RESEND_WAITLIST_AUDIENCE_ID'];
  if (!resend || !audienceId) return;

  try {
    await resend.contacts.create({ email, audienceId });
  } catch (error) {
    console.error(
      'Failed to add to waitlist audience:',
      error instanceof Error ? error.message : error
    );
  }
}

export async function moveToApprovedAudience(email: string): Promise<void> {
  const resend = getResendClient();
  const waitlistId = process.env['RESEND_WAITLIST_AUDIENCE_ID'];
  const betaId = process.env['RESEND_BETA_AUDIENCE_ID'];
  if (!resend) return;

  // Remove from waitlist audience (best-effort)
  if (waitlistId) {
    try {
      const contacts = await resend.contacts.list({ audienceId: waitlistId });
      const contact = contacts.data?.data?.find((c: { email: string }) => c.email === email);
      if (contact) {
        await resend.contacts.remove({ id: contact.id, audienceId: waitlistId });
      }
    } catch (error) {
      console.error(
        'Failed to remove from waitlist audience:',
        error instanceof Error ? error.message : error
      );
    }
  }

  // Add to beta-users audience (best-effort)
  if (betaId) {
    try {
      await resend.contacts.create({ email, audienceId: betaId });
    } catch (error) {
      console.error(
        'Failed to add to beta audience:',
        error instanceof Error ? error.message : error
      );
    }
  }
}

export async function removeFromBetaAudience(email: string): Promise<void> {
  const resend = getResendClient();
  const betaId = process.env['RESEND_BETA_AUDIENCE_ID'];
  if (!resend || !betaId) return;

  try {
    const contacts = await resend.contacts.list({ audienceId: betaId });
    const contact = contacts.data?.data?.find((c: { email: string }) => c.email === email);
    if (contact) {
      await resend.contacts.remove({ id: contact.id, audienceId: betaId });
    }
  } catch (error) {
    console.error(
      'Failed to remove from beta audience:',
      error instanceof Error ? error.message : error
    );
  }
}
