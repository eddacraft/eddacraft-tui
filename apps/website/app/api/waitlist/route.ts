import { neon } from '@neondatabase/serverless';
import { NextResponse } from 'next/server';
import { sendWaitlistConfirmation } from '@/lib/email';

export const dynamic = 'force-dynamic';

export async function POST(request: Request) {
  try {
    const databaseUrl = process.env.DATABASE_URL;
    if (!databaseUrl) {
      console.error('DATABASE_URL not configured');
      return NextResponse.json({ error: 'Service unavailable' }, { status: 503 });
    }

    const contentType = request.headers.get('content-type') || '';
    if (!contentType.toLowerCase().includes('application/json')) {
      return NextResponse.json({ error: 'Content-Type must be application/json' }, { status: 400 });
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

    const trimmedEmail = email.trim();
    if (trimmedEmail.length > 254) {
      return NextResponse.json({ error: 'Invalid email format' }, { status: 400 });
    }
    const emailRegex =
      /^[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-zA-Z0-9!#$%&'*+/=?^_`{|}~-]+)*@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*\.[a-zA-Z]{2,}$/;
    if (!emailRegex.test(trimmedEmail)) {
      return NextResponse.json({ error: 'Invalid email format' }, { status: 400 });
    }

    const sql = neon(databaseUrl);
    const result = (await sql`
      INSERT INTO waitlist (email, source)
      VALUES (${trimmedEmail.toLowerCase()}, 'website')
      ON CONFLICT (email) DO UPDATE SET updated_at = NOW()
      RETURNING id, email, created_at, (xmax = 0) AS is_new
    `) as { id: number; email: string; created_at: string; is_new: boolean }[];

    if (!Array.isArray(result) || result.length === 0) {
      console.error('Waitlist insertion did not return a result');
      return NextResponse.json({ error: 'Failed to join waitlist' }, { status: 500 });
    }

    // Only send confirmation for genuinely new signups, not repeated submissions
    if (result[0].is_new) {
      try {
        await sendWaitlistConfirmation(result[0].email);
      } catch (err) {
        console.error('Waitlist confirmation email error:', err);
      }
    }

    return NextResponse.json({
      success: true,
      message: 'Added to waitlist',
      email: result[0].email,
    });
  } catch (error: unknown) {
    if (error instanceof Error) {
      console.error('Waitlist submission error:', error.message);
    } else {
      console.error('Waitlist submission error:', error);
    }
    return NextResponse.json({ error: 'Failed to join waitlist' }, { status: 500 });
  }
}
