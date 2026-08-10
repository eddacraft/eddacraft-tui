'use client';

import React, { useState, useRef, useEffect } from 'react';

const ANVIL_VERSION = 'v0.9.4';
const ANVIL_BUILD_HASH = '165d33';

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
  emailStatus?: string;
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
      emailStatus?: string;
      isNewSignup?: boolean;
    };
    if (!response.ok) {
      return {
        success: false,
        error: data.error || 'Failed to join waitlist',
        warning: data.warning,
        emailSent: data.emailSent,
        emailStatus: data.emailStatus,
        isNewSignup: data.isNewSignup,
      };
    }

    return {
      success: true,
      warning: data.warning,
      emailSent: data.emailSent,
      emailStatus: data.emailStatus,
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
    { text: 'Verifying...', colorClass: 'text-text-muted', delay: 600 },
    { text: '[ OK ] Access request received', colorClass: 'text-edda', delay: 400 },
    {
      text:
        submitWarning && submitWarning.includes('WARN')
          ? `Welcome aboard. Access is queued for ${userEmail}`
          : `Welcome aboard. We'll be in touch at ${userEmail}`,
      colorClass: 'text-text-muted',
      delay: 0,
    },
  ];
  if (emailFailed) {
    lines.push({
      text: '[ WARN ] Confirmation email could not be sent — you are still on the list',
      colorClass: 'text-amber-400',
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
  const showFinalCursor =
    submitted && currentLineIndex >= buildResponseLines(email, submitWarning, emailFailed).length;

  // Typewriter effect
  useEffect(() => {
    if (!submitted) return;

    const responseLines = buildResponseLines(email, submitWarning, emailFailed);

    if (currentLineIndex >= responseLines.length) {
      return;
    }

    const currentLine = responseLines[currentLineIndex];
    const fullText = currentLine.text;

    if (currentCharIndex < fullText.length) {
      const timeout = setTimeout(() => {
        setDisplayedLines((prev) => {
          const newLines = [...prev];
          if (!newLines[currentLineIndex]) {
            newLines[currentLineIndex] = {
              id: currentLine.text,
              text: '',
              colorClass: currentLine.colorClass,
            };
          }
          newLines[currentLineIndex] = {
            ...newLines[currentLineIndex],
            text: fullText.slice(0, currentCharIndex + 1),
          };
          return newLines;
        });
        setCurrentCharIndex((prev) => prev + 1);
      }, 25);
      return () => clearTimeout(timeout);
    } else {
      const timeout = setTimeout(() => {
        setCurrentLineIndex((prev) => prev + 1);
        setCurrentCharIndex(0);
      }, currentLine.delay);
      return () => clearTimeout(timeout);
    }
  }, [submitted, currentLineIndex, currentCharIndex, email, submitWarning, emailFailed]);

  const handleSubmit = async (e?: React.FormEvent | React.MouseEvent) => {
    e?.preventDefault();
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

  const handleTerminalClick = () => {
    if (!submitted && inputRef.current) {
      inputRef.current.focus();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape' && !submitted) {
      setEmail('');
      inputRef.current?.focus();
    }
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
  };

  return (
    <footer id="waitlist" className="border-t border-structure bg-void font-mono">
      <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12 sm:py-16 lg:py-24">
        <div className="flex flex-col items-center justify-center space-y-6 sm:space-y-8">
          {/* Terminal Section */}
          <div className="w-full max-w-xl space-y-4">
            {/* Header Messages */}
            <div className="text-xs sm:text-sm text-center space-y-1">
              <p className="text-text-muted">Engineering team onboarding in progress.</p>
              <p className="text-text-primary">Cohort capacity is limited.</p>
            </div>

            {/* Interactive Terminal Box */}
            <div
              onClick={handleTerminalClick}
              className="bg-surface border border-structure px-4 py-3 space-y-2 cursor-text text-xs sm:text-sm"
            >
              {/* Status/Context Line */}
              <div className="flex items-center gap-2 text-text-muted">
                <span className="text-edda">-&gt;</span>
                <span className="text-anvil">~/eddacraft/anvil</span>
                <span className="text-structure">.</span>
                <span>main</span>
              </div>

              {/* Input Line */}
              <form
                onSubmit={handleSubmit}
                onKeyDown={handleKeyDown}
                className="flex items-center gap-3"
              >
                <span className="text-text-muted">$</span>
                <span className="text-text-primary">request access</span>
                {!submitted ? (
                  <>
                    <input
                      ref={inputRef}
                      type="email"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      placeholder="you@example.dev"
                      className="flex-1 min-w-0 bg-transparent text-text-primary placeholder:text-text-muted/50 outline-none border-none"
                      autoComplete="email"
                      disabled={isSubmitting}
                    />
                    {isSubmitting ? (
                      <span className="text-text-muted animate-pulse">...</span>
                    ) : (
                      <span className="inline-block w-[0.6ch] h-[1.1em] bg-anvil/70 animate-pulse"></span>
                    )}
                  </>
                ) : (
                  <span className="text-text-muted">{email}</span>
                )}
              </form>

              {/* Error Display */}
              {submitError && (
                <div className="flex items-center gap-3">
                  <span className="text-text-muted opacity-0">$</span>
                  <span className="text-red-500">[ ERROR ] {submitError}</span>
                </div>
              )}

              {/* Warning Display */}
              {submitted && submitWarning && (
                <div className="flex items-center gap-3">
                  <span className="text-text-muted opacity-0">$</span>
                  <span className="text-amber-400">{submitWarning}</span>
                </div>
              )}

              {/* Response Lines with Typewriter */}
              {displayedLines.map((line, index) => (
                <div key={line.id} className="flex items-center gap-3">
                  <span className="text-text-muted opacity-0">$</span>
                  <span className={line.colorClass}>{line.text}</span>
                  {index === currentLineIndex &&
                    currentCharIndex <
                      buildResponseLines(email, submitWarning, emailFailed)[index]?.text.length && (
                      <span className="inline-block w-[0.6ch] h-[1.1em] bg-anvil/70 animate-pulse"></span>
                    )}
                </div>
              ))}

              {/* Final Prompt */}
              {showFinalCursor && (
                <div className="flex items-center gap-3 mt-2">
                  <span className="text-text-muted">$</span>
                  <span className="inline-block w-[0.6ch] h-[1.1em] bg-anvil/70 animate-pulse"></span>
                </div>
              )}
            </div>

            {/* Instructions / Reset */}
            <div className="text-center text-[10px] sm:text-xs text-text-muted">
              {!submitted ? (
                <p>
                  <button
                    type="button"
                    onClick={handleSubmit}
                    className="bg-transparent border-none p-0 font-mono text-[10px] sm:text-xs text-text-muted cursor-pointer hover:text-text-primary transition-colors"
                  >
                    <span className="text-text-primary">enter</span> submit
                  </button>
                  <span className="text-structure mx-2">::</span>
                  <span className="text-text-primary">esc</span> clear
                </p>
              ) : showFinalCursor ? (
                <button
                  onClick={reset}
                  className="text-text-muted hover:text-text-primary transition-colors"
                >
                  [ reset ]
                </button>
              ) : null}
            </div>
          </div>

          {/* System Bar - Build Artifact Style */}
          <div className="flex items-center gap-4 sm:gap-6 pt-6 sm:pt-8 border-t border-structure w-full justify-center flex-wrap text-[10px] sm:text-xs uppercase tracking-wide">
            <button
              onClick={() => setShowPreReleaseModal(true)}
              className="flex items-center gap-2 text-text-muted hover:text-edda transition-colors"
            >
              <span>LATEST:</span>
              <span className="bg-structure text-text-primary px-1.5 py-0.5 rounded-sm text-[9px] sm:text-[10px]">
                {ANVIL_VERSION}
              </span>
              <span className="text-structure">::</span>
              <span className="bg-structure text-text-primary px-1.5 py-0.5 rounded-sm text-[9px] sm:text-[10px]">
                {ANVIL_BUILD_HASH}
              </span>
            </button>

            <a
              href="/security"
              className="flex items-center gap-2 text-text-muted hover:text-edda transition-colors"
            >
              <span>ADVISORIES:</span>
              <span className="text-edda">NONE</span>
            </a>

            <a
              href="https://github.com/eddacraft"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 text-text-muted hover:text-edda transition-colors"
            >
              <span>GITHUB:</span>
              <span className="text-text-primary">eddacraft</span>
            </a>

            <a href="/privacy" className="text-text-muted hover:text-edda transition-colors">
              PRIVACY
            </a>

            <span className="text-text-muted/30">{'// (c) 2026 eddacraft, Inc.'}</span>
          </div>
        </div>
      </div>

      {/* Pre-Release Modal */}
      {showPreReleaseModal && (
        <div
          className="fixed inset-0 bg-void/90 flex items-center justify-center z-50 p-4"
          onClick={() => setShowPreReleaseModal(false)}
        >
          <div
            className="bg-surface border border-structure max-w-md w-full p-6 space-y-6"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div className="text-xs text-text-muted uppercase tracking-wide">
              {'// PRE-RELEASE_NOTICE'}
            </div>

            {/* Modal Content */}
            <div className="space-y-4">
              <h3 className="text-lg sm:text-xl text-anvil tracking-tight">
                anvil is in pre-release
              </h3>
              <p className="text-sm text-text-muted leading-relaxed">
                We are onboarding engineering teams in controlled cohorts. Request access below to
                join the next available slot.
              </p>
            </div>

            {/* Modal Actions */}
            <div className="flex flex-col sm:flex-row gap-3 pt-2">
              <button
                onClick={() => {
                  setShowPreReleaseModal(false);
                  setTimeout(() => {
                    inputRef.current?.focus();
                  }, 100);
                }}
                className="flex-1 border border-anvil bg-anvil/5 px-4 py-3 text-xs sm:text-sm text-anvil hover:bg-anvil/10 transition-colors uppercase tracking-wide"
              >
                Request Access
              </button>
              <button
                onClick={() => setShowPreReleaseModal(false)}
                className="flex-1 border border-structure px-4 py-3 text-xs sm:text-sm text-text-muted hover:text-text-primary hover:border-text-muted transition-colors uppercase tracking-wide"
              >
                Close
              </button>
            </div>

            {/* Version Info */}
            <div className="text-[10px] text-text-muted/50 pt-2 border-t border-structure">
              build: {ANVIL_VERSION} :: {ANVIL_BUILD_HASH} :: pre-release
            </div>
          </div>
        </div>
      )}
    </footer>
  );
}
