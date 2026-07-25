import type { Metadata } from 'next';

export const metadata: Metadata = { title: 'Access Pending — eddacraft Docs' };

const linkStyle = { color: '#60a5fa' };

export default function PendingPage() {
  return (
    <main
      style={{
        fontFamily: 'system-ui, sans-serif',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100vh',
        margin: 0,
        background: '#0a0a0a',
        color: '#e5e5e5',
      }}
    >
      <div style={{ textAlign: 'left', maxWidth: 560, padding: '2rem' }}>
        <h1 style={{ textAlign: 'center' }}>Access Pending</h1>
        <p>
          Your GitHub sign-in didn&apos;t match an approved anvil beta account. There are two common
          causes:
        </p>
        <h2 style={{ fontSize: '1.05rem', marginTop: '1.5rem' }}>Waiting for an invite</h2>
        <p>
          If you&apos;ve joined the waitlist but haven&apos;t been approved yet, you&apos;ll receive
          an email when your slot opens.
        </p>
        <h2 style={{ fontSize: '1.05rem', marginTop: '1.5rem' }}>
          Already a beta user, but your GitHub email doesn&apos;t match?
        </h2>
        <p>
          Your GitHub account may not have the email address you registered with. Add it at{' '}
          <a href="https://github.com/settings/emails" style={linkStyle}>
            github.com/settings/emails
          </a>
          , click the verification link GitHub sends, then try signing in again.
        </p>
        <p style={{ marginTop: '1.5rem' }}>
          Need to change the email you signed up with, or stuck? Email{' '}
          <a href="mailto:help@eddacraft.ai" style={linkStyle}>
            help@eddacraft.ai
          </a>
          .
        </p>
        <p style={{ textAlign: 'center', marginTop: '2rem' }}>
          <a href="/auth/login" style={{ ...linkStyle, marginRight: '1rem' }}>
            Try again
          </a>
          <a href="/" style={linkStyle}>
            Return home
          </a>
        </p>
      </div>
    </main>
  );
}
