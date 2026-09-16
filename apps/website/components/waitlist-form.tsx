'use client';

import { useState, type FormEvent, type RefObject } from 'react';

import { buildResponseLines, isValidEmail, submitToWaitlist } from '@/lib/waitlist';

type FormStatus = 'idle' | 'loading' | 'success' | 'error';

interface WaitlistFormProps {
  /** Unique input id; the footer form owns `waitlist-email`. */
  id: string;
  inputRef?: RefObject<HTMLInputElement | null>;
}

// Compact, animation-free waitlist form for the hero. The footer keeps its
// typewriter CLI treatment; both share the submit path in lib/waitlist.ts.
export function WaitlistForm({ id, inputRef }: WaitlistFormProps) {
  const [email, setEmail] = useState('');
  const [status, setStatus] = useState<FormStatus>('idle');
  const [errorMessage, setErrorMessage] = useState('');
  const [submitWarning, setSubmitWarning] = useState<string | null>(null);
  const [emailFailed, setEmailFailed] = useState(false);

  const submitted = status === 'success';
  const responseLines = submitted
    ? buildResponseLines(email, submitWarning, emailFailed).filter(
        (line) => line.text !== 'Verifying...'
      )
    : [];

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (status === 'loading') return;

    const trimmedEmail = email.trim();
    if (!isValidEmail(trimmedEmail)) {
      setStatus('error');
      setErrorMessage('Enter a valid work email');
      return;
    }

    setStatus('loading');
    setErrorMessage('');
    setSubmitWarning(null);

    const result = await submitToWaitlist(trimmedEmail);
    if (result.success) {
      setEmail(trimmedEmail);
      setSubmitWarning(result.warning ?? null);
      setEmailFailed(result.emailSent === false && result.isNewSignup === true);
      setStatus('success');
      return;
    }

    setStatus('error');
    setErrorMessage(result.error ?? 'Something went wrong');
  };

  const reset = () => {
    setEmail('');
    setStatus('idle');
    setErrorMessage('');
    setSubmitWarning(null);
    setEmailFailed(false);
    window.setTimeout(() => inputRef?.current?.focus(), 0);
  };

  return (
    <div className="max-w-xl font-mono text-xs sm:text-sm">
      <form onSubmit={handleSubmit} noValidate className="flex flex-col gap-3 sm:flex-row">
        <label htmlFor={id} className="sr-only">
          Work email
        </label>
        <div className="flex min-w-0 flex-1 items-center gap-3 border border-structure bg-surface px-3 py-3 focus-within:border-anvil">
          <span className="text-anvil">$</span>
          <span className="whitespace-nowrap text-off-white">request access</span>
          {!submitted ? (
            <input
              id={id}
              ref={inputRef}
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="you@example.dev"
              autoComplete="email"
              disabled={status === 'loading'}
              aria-invalid={status === 'error' ? true : undefined}
              className="min-w-0 flex-1 border-none bg-transparent text-off-white outline-none placeholder:text-ghost-grey/60 disabled:opacity-50"
            />
          ) : (
            <span className="min-w-0 flex-1 truncate text-ghost-grey">{email}</span>
          )}
        </div>
        {!submitted ? (
          <button
            type="submit"
            disabled={status === 'loading'}
            className="shrink-0 border border-anvil bg-anvil px-5 py-3 text-xs uppercase tracking-wide text-void transition-colors hover:bg-anvil/90 disabled:opacity-50"
          >
            {status === 'loading' ? 'sending...' : '[ = ] request early access'}
          </button>
        ) : null}
      </form>

      {status === 'error' ? (
        <p className="mt-3 text-brick-red" role="alert">
          [ ERR ] {errorMessage}
        </p>
      ) : null}

      <div aria-live="polite">
        {submitted ? (
          <div className="mt-3 space-y-1">
            {responseLines.map((line) => (
              <p key={line.text} className={line.colorClass}>
                {line.text}
              </p>
            ))}
            {submitWarning ? <p className="text-dull-amber">{submitWarning}</p> : null}
            <button
              type="button"
              onClick={reset}
              className="mt-1 text-ghost-grey hover:text-off-white"
            >
              [ reset ]
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
