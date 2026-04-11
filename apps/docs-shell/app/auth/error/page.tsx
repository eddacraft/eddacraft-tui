import type { Metadata } from 'next';

export const metadata: Metadata = { title: 'Sign-in error — EddaCraft Docs' };

const REASONS: Record<string, string> = {
  denied: 'You cancelled the GitHub sign-in.',
  oauth_error: 'GitHub returned an OAuth error.',
  missing_params: 'The callback URL is missing required parameters.',
  invalid_state: 'The OAuth state parameter was invalid or tampered with.',
  csrf_mismatch: 'CSRF nonce did not match. Please try signing in again.',
  api_error: 'Could not reach the authentication service.',
  auth_failed: 'Authentication failed.',
  invalid_response: 'The authentication service returned an unexpected response.',
};

export default async function ErrorPage({
  searchParams,
}: {
  searchParams: Promise<{ reason?: string }>;
}) {
  const { reason } = await searchParams;
  const message = (reason && REASONS[reason]) ?? 'An unknown error occurred.';

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
        <h1>Sign-in error</h1>
        <p>{message}</p>
        <p>
          <a href="/auth/login" style={{ color: '#60a5fa', marginRight: '1rem' }}>
            Try again
          </a>
          <a href="/" style={{ color: '#60a5fa' }}>
            Return home
          </a>
        </p>
      </div>
    </main>
  );
}
