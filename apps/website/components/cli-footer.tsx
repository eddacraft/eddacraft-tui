'use client';

import { useEffect, useMemo, useRef, useState } from 'react';

const ANVIL_VERSION = 'v0.9.5-beta';
const ANVIL_BUILD_HASH = '5c4b61a';

interface ResponseLine {
  text: string;
  colorClass: string;
  delay: number;
}

interface DisplayedLine {
  id: string;
  text: string;
  colorClass: string;
}

interface WaitlistSubmitResult {
  success: boolean;
  error?: string;
  warning?: string;
  emailSent?: boolean;
  isNewSignup?: boolean;
}

async function submitToWaitlist(email: string): Promise<WaitlistSubmitResult> {
  try {
    const apiBase = (process.env.NEXT_PUBLIC_API_URL ?? 'https://api.eddacraft.ai').replace(
      /\/+$/,
      ''
    );
    const response = await fetch(`${apiBase}/api/v1/waitlist`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email }),
    });
    const data = (await response.json()) as {
      error?: string;
      warning?: string;
      emailSent?: boolean;
      isNewSignup?: boolean;
    };
    if (!response.ok) {
      return {
        success: false,
        error: data.error || 'Failed to join waitlist',
        warning: data.warning,
        emailSent: data.emailSent,
        isNewSignup: data.isNewSignup,
      };
    }

    return {
      success: true,
      warning: data.warning,
      emailSent: data.emailSent,
      isNewSignup: data.isNewSignup,
    };
  } catch {
    return { success: false, error: 'Network error. Please try again.' };
  }
}

function buildResponseLines(
  userEmail: string,
  submitWarning: string | null,
  emailFailed: boolean
): ResponseLine[] {
  const lines: ResponseLine[] = [
    { text: 'Verifying...', colorClass: 'text-ghost-grey', delay: 600 },
    { text: '[ OK ] Access request received', colorClass: 'text-edda', delay: 400 },
    {
      text:
        submitWarning && submitWarning.includes('WARN')
          ? `Access is queued for ${userEmail}`
          : `We will be in touch at ${userEmail}`,
      colorClass: 'text-ghost-grey',
      delay: 0,
    },
  ];
  if (emailFailed) {
    lines.push({
      text: '[ WARN ] Confirmation email could not be sent — you are still on the list',
      colorClass: 'text-dull-amber',
      delay: 300,
    });
  }
  return lines;
}

export function CLIFooter() {
  const [email, setEmail] = useState('');
  const [submitted, setSubmitted] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [submitWarning, setSubmitWarning] = useState<string | null>(null);
  const [emailFailed, setEmailFailed] = useState(false);
  const [displayedLines, setDisplayedLines] = useState<DisplayedLine[]>([]);
  const [currentLineIndex, setCurrentLineIndex] = useState(0);
  const [currentCharIndex, setCurrentCharIndex] = useState(0);
  const [showPreReleaseModal, setShowPreReleaseModal] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const responseLines = useMemo(
    () => buildResponseLines(email, submitWarning, emailFailed),
    [email, submitWarning, emailFailed]
  );
  const showFinalCursor = submitted && currentLineIndex >= responseLines.length;

  useEffect(() => {
    if (!submitted || currentLineIndex >= responseLines.length) return;

    const currentLine = responseLines[currentLineIndex];
    const fullText = currentLine.text;

    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      const reducedMotionTimeout = window.setTimeout(() => {
        setDisplayedLines(
          responseLines.map((line) => ({
            id: line.text,
            text: line.text,
            colorClass: line.colorClass,
          }))
        );
        setCurrentLineIndex(responseLines.length);
      }, 0);
      return () => window.clearTimeout(reducedMotionTimeout);
    }

    if (currentCharIndex < fullText.length) {
      const timeout = window.setTimeout(() => {
        setDisplayedLines((previous) => {
          const next = [...previous];
          next[currentLineIndex] = {
            id: currentLine.text,
            text: fullText.slice(0, currentCharIndex + 1),
            colorClass: currentLine.colorClass,
          };
          return next;
        });
        setCurrentCharIndex((previous) => previous + 1);
      }, 25);
      return () => window.clearTimeout(timeout);
    }

    const timeout = window.setTimeout(() => {
      setCurrentLineIndex((previous) => previous + 1);
      setCurrentCharIndex(0);
    }, currentLine.delay);
    return () => window.clearTimeout(timeout);
  }, [submitted, currentLineIndex, currentCharIndex, responseLines]);

  const handleSubmit = async (event?: React.FormEvent | React.MouseEvent) => {
    event?.preventDefault();
    const trimmedEmail = email.trim();
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(trimmedEmail) || isSubmitting) return;

    setIsSubmitting(true);
    setSubmitError(null);
    setSubmitWarning(null);

    const result = await submitToWaitlist(trimmedEmail);
    if (result.success) {
      setSubmitWarning(result.warning || null);
      setEmailFailed(result.emailSent === false && result.isNewSignup === true);
      setSubmitted(true);
    } else {
      setSubmitError(result.error || 'Something went wrong');
    }
    setIsSubmitting(false);
  };

  const reset = () => {
    setEmail('');
    setSubmitted(false);
    setIsSubmitting(false);
    setSubmitError(null);
    setSubmitWarning(null);
    setEmailFailed(false);
    setDisplayedLines([]);
    setCurrentLineIndex(0);
    setCurrentCharIndex(0);
    window.setTimeout(() => inputRef.current?.focus(), 0);
  };

  return (
    <footer id="waitlist" className="site-section bg-surface font-mono">
      <div className="site-container grid grid-cols-1 gap-8 py-14 lg:grid-cols-[0.75fr_1.25fr] lg:items-center">
        <div className="min-w-0">
          <p className="section-label mb-5">{'// EARLY_ACCESS'}</p>
          <h2 className="text-3xl leading-tight text-off-white sm:text-4xl">
            BUILD WITH SPEED.
            <br />
            <span className="text-anvil">SHIP WITH INTEGRITY.</span>
          </h2>
          <p className="mt-5 max-w-md font-sans text-sm leading-6 text-ghost-grey">
            Get early access to anvil and help shape trustworthy AI-assisted software engineering.
          </p>
        </div>

        <div className="min-w-0 border border-structure bg-void p-4 text-xs sm:p-5 sm:text-sm">
          <div className="mb-4 flex items-center gap-2 text-ghost-grey">
            <span className="text-edda">-&gt;</span>
            <span className="text-anvil">~/eddacraft/anvil</span>
            <span className="text-structure">.</span>
            <span>main</span>
          </div>

          <form onSubmit={handleSubmit} className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <label htmlFor="waitlist-email" className="sr-only">
              Work email
            </label>
            <div className="flex min-w-0 flex-1 items-center gap-3 border border-structure bg-surface px-3 py-3 focus-within:border-anvil">
              <span className="text-anvil">$</span>
              <span className="whitespace-nowrap text-off-white">request access</span>
              {!submitted ? (
                <input
                  id="waitlist-email"
                  ref={inputRef}
                  type="email"
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Escape') {
                      setEmail('');
                      inputRef.current?.focus();
                    }
                  }}
                  placeholder="you@example.dev"
                  className="min-w-0 flex-1 border-none bg-transparent text-off-white outline-none placeholder:text-ghost-grey/60"
                  autoComplete="email"
                  disabled={isSubmitting}
                />
              ) : (
                <span className="min-w-0 flex-1 truncate text-ghost-grey">{email}</span>
              )}
            </div>
            {!submitted ? (
              <button
                type="submit"
                disabled={isSubmitting}
                className="border border-anvil bg-anvil px-5 py-3 text-xs uppercase tracking-wide text-void transition-colors hover:bg-anvil/90 disabled:opacity-50"
              >
                {isSubmitting ? 'sending...' : '[ = ] request access'}
              </button>
            ) : null}
          </form>

          {submitError ? (
            <p className="mt-3 text-brick-red" role="alert">
              [ ERR ] {submitError}
            </p>
          ) : null}
          {submitted && submitWarning ? (
            <p className="mt-3 text-dull-amber">{submitWarning}</p>
          ) : null}

          {displayedLines.length > 0 ? (
            <div className="mt-4 space-y-1 border-t border-structure pt-4" aria-live="polite">
              {displayedLines.map((line) => (
                <p key={line.id} className={line.colorClass}>
                  {line.text}
                </p>
              ))}
              {showFinalCursor ? (
                <button
                  type="button"
                  onClick={reset}
                  className="mt-2 text-ghost-grey hover:text-off-white"
                >
                  [ reset ]
                </button>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>

      <div className="border-t border-structure">
        <div className="site-container flex flex-col gap-4 py-5 text-[10px] uppercase tracking-wide text-ghost-grey sm:flex-row sm:items-center sm:justify-between">
          <div className="flex flex-wrap gap-5">
            <a href="https://docs.eddacraft.ai" className="hover:text-off-white">
              docs
            </a>
            <a href="https://github.com/eddacraft" className="hover:text-off-white">
              github
            </a>
            <a href="/security" className="hover:text-off-white">
              security
            </a>
            <a href="/privacy" className="hover:text-off-white">
              privacy
            </a>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <button
              type="button"
              onClick={() => setShowPreReleaseModal(true)}
              className="hover:text-edda"
            >
              {ANVIL_VERSION} :: {ANVIL_BUILD_HASH}
            </button>
            <span>© 2026 eddacraft</span>
          </div>
        </div>
      </div>

      {showPreReleaseModal ? (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-void/90 p-4"
          onClick={() => setShowPreReleaseModal(false)}
        >
          <div
            className="w-full max-w-md space-y-5 border border-structure bg-surface p-6"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="text-xs uppercase tracking-wide text-ghost-grey">
              {'// PRE_RELEASE_NOTICE'}
            </div>
            <h3 className="text-xl text-anvil">anvil is in pre-release</h3>
            <p className="font-sans text-sm leading-6 text-ghost-grey">
              Engineering teams are onboarding in controlled cohorts. Request access to join the
              next available slot.
            </p>
            <button
              type="button"
              onClick={() => {
                setShowPreReleaseModal(false);
                window.setTimeout(() => inputRef.current?.focus(), 100);
              }}
              className="border border-anvil bg-anvil/5 px-4 py-3 text-xs uppercase tracking-wide text-anvil hover:bg-anvil/10"
            >
              Request access
            </button>
          </div>
        </div>
      ) : null}
    </footer>
  );
}
