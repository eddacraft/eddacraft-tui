import { NextResponse } from 'next/server';
import { sendWaitlistConfirmation } from '@/lib/email';

export const dynamic = 'force-dynamic';

function isAuthorized(request: Request): boolean {
  const expectedToken = process.env.WAITLIST_RESEND_ADMIN_TOKEN;
  if (!expectedToken) return false;

  const authHeader = request.headers.get('authorization');
  const bearer = authHeader?.startsWith('Bearer ') ? authHeader.slice(7).trim() : null;
  const direct = request.headers.get('x-waitlist-admin-token')?.trim();
  const provided = bearer || direct;

  return provided === expectedToken;
}

export async function POST(request: Request) {
  if (!isAuthorized(request)) {
    return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
  }

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: 'Invalid JSON payload' }, { status: 400 });
  }

  const { email } = body as { email?: unknown };
  if (!email || typeof email !== 'string') {
    return NextResponse.json({ error: 'Email is required' }, { status: 400 });
  }

  const trimmedEmail = email.trim().toLowerCase();
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(trimmedEmail)) {
    return NextResponse.json({ error: 'Invalid email format' }, { status: 400 });
  }

  const delivery = await sendWaitlistConfirmation(trimmedEmail);

  if (!delivery.sent) {
    return NextResponse.json(
      {
        success: false,
        email: trimmedEmail,
        emailSent: false,
        emailStatus: delivery.code || 'failed',
        error: delivery.message || 'Failed to send confirmation email',
      },
      { status: 502 }
    );
  }

  return NextResponse.json({
    success: true,
    email: trimmedEmail,
    emailSent: true,
    emailStatus: 'sent',
  });
}
