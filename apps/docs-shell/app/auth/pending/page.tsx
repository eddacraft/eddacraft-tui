import type { Metadata } from 'next';

export const metadata: Metadata = { title: 'Access Pending — eddacraft Docs' };

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
      <div style={{ textAlign: 'center', maxWidth: 480, padding: '2rem' }}>
        <h1>Access Pending</h1>
        <p>
          Your GitHub account has been registered, but access to Anvil documentation requires
          approval.
        </p>
        <p>You&apos;ll receive an email once your access has been approved.</p>
        <p>
          <a href="/" style={{ color: '#60a5fa' }}>
            Return to docs home
          </a>
        </p>
      </div>
    </main>
  );
}
