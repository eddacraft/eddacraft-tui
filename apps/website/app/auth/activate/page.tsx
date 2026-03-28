'use client';

import { Suspense, useState, type FormEvent } from 'react';
import { useSearchParams } from 'next/navigation';
import Link from 'next/link';

type Status = 'idle' | 'loading' | 'success' | 'error';

export default function ActivatePage() {
  return (
    <Suspense fallback={null}>
      <ActivateForm />
    </Suspense>
  );
}

function ActivateForm() {
  const searchParams = useSearchParams();
  const [email, setEmail] = useState('');
  const [code, setCode] = useState(searchParams.get('code') ?? '');
  const [status, setStatus] = useState<Status>('idle');
  const [errorMessage, setErrorMessage] = useState('');

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    setStatus('loading');
    setErrorMessage('');

    try {
      const apiBase = process.env.NEXT_PUBLIC_API_URL ?? '';
      const res = await fetch(`${apiBase}/api/v1/auth/device/confirm`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          userCode: code.trim().toUpperCase(),
          email: email.trim().toLowerCase(),
        }),
      });

      if (res.ok) {
        setStatus('success');
      } else {
        const data = await res.json().catch(() => ({}));
        setErrorMessage(data.error ?? 'Invalid or expired code');
        setStatus('error');
      }
    } catch {
      setErrorMessage('Network error — please try again');
      setStatus('error');
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-void font-mono text-text-primary">
      <div className="w-full max-w-md px-6">
        {/* Header */}
        <div className="mb-8">
          <Link
            href="/"
            className="text-xs text-text-muted transition-colors hover:text-text-primary"
          >
            {'<-'} back to anvil
          </Link>
        </div>

        <h1 className="mb-8 text-sm text-text-muted">
          <span className="text-edda">$</span> anvil :: activate
        </h1>

        {status === 'success' ? (
          <div className="space-y-4">
            <p className="text-sm text-edda">Device confirmed — return to your terminal.</p>
            <p className="text-xs text-text-muted">You can close this window.</p>
          </div>
        ) : (
          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Email */}
            <div className="space-y-2">
              <label htmlFor="email" className="block text-xs text-text-muted">
                Enter your email:
              </label>
              <input
                id="email"
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={status === 'loading'}
                placeholder="you@example.com"
                className="w-full border border-structure bg-surface px-3 py-2 font-mono text-sm text-text-primary placeholder:text-text-muted/40 focus:border-edda focus:outline-none disabled:opacity-50"
              />
            </div>

            {/* Code */}
            <div className="space-y-2">
              <label htmlFor="code" className="block text-xs text-text-muted">
                Enter activation code:
              </label>
              <input
                id="code"
                type="text"
                required
                autoComplete="off"
                value={code}
                onChange={(e) => setCode(e.target.value.toUpperCase())}
                disabled={status === 'loading'}
                placeholder="ANVIL-XXXX"
                className="w-full border border-structure bg-surface px-3 py-2 font-mono text-sm tracking-wider text-text-primary placeholder:text-text-muted/40 focus:border-edda focus:outline-none disabled:opacity-50"
              />
            </div>

            {/* Error */}
            {status === 'error' && (
              <p className="text-xs text-anvil" role="alert">
                {errorMessage}
              </p>
            )}

            {/* Submit */}
            <button
              type="submit"
              disabled={status === 'loading'}
              className="border border-edda px-4 py-2 text-sm text-edda transition-colors hover:bg-edda/10 disabled:opacity-50"
            >
              {status === 'loading' ? 'Confirming...' : '[ Confirm ]'}
            </button>
          </form>
        )}
      </div>
    </main>
  );
}
