import { neon } from '@neondatabase/serverless';
import { NextResponse } from 'next/server';

export const dynamic = 'force-dynamic';

export async function POST(request: Request) {
  try {
    const databaseUrl = process.env.DATABASE_URL;
    if (!databaseUrl) {
      console.error('DATABASE_URL not configured');
      return NextResponse.json({ error: 'Service unavailable' }, { status: 503 });
    }

    const sql = neon(databaseUrl);
    const { email } = await request.json();

    if (!email || typeof email !== 'string') {
      return NextResponse.json({ error: 'Email is required' }, { status: 400 });
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(email)) {
      return NextResponse.json({ error: 'Invalid email format' }, { status: 400 });
    }

    const result = (await sql`
      INSERT INTO waitlist (email, source)
      VALUES (${email.toLowerCase().trim()}, 'website')
      ON CONFLICT (email) DO UPDATE SET updated_at = NOW()
      RETURNING id, email, created_at
    `) as { id: number; email: string; created_at: string }[];

    return NextResponse.json({
      success: true,
      message: 'Added to waitlist',
      email: result[0]?.email,
    });
  } catch (error) {
    console.error('Waitlist submission error:', error);
    return NextResponse.json({ error: 'Failed to join waitlist' }, { status: 500 });
  }
}
