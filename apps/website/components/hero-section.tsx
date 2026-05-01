'use client';

import { useState, type FormEvent } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import Link from 'next/link';
import { TerminalWindow } from './terminal-window';

const REDACTED_INSTALL_COMMAND = 'brew install eddacraft/[EARLY-ACCESS]/anvil';

type AccessStatus = 'idle' | 'loading' | 'success' | 'error';

interface InstallUnlockResponse {
  command?: string;
  error?: string;
}

type UnlockResult =
  | { ok: true; command: string }
  | { ok: false; reason: 'invalid_key' | 'service_unavailable' | 'unknown' };

async function unlockInstallCommand(accessKey: string): Promise<UnlockResult> {
  const response = await fetch('/api/early-access/install', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ accessKey }),
  });

  const data = (await response.json().catch(() => ({}))) as InstallUnlockResponse;

  if (response.ok && typeof data.command === 'string') {
    return { ok: true, command: data.command };
  }

  if (response.status === 503 || data.error === 'access_service_unavailable') {
    return { ok: false, reason: 'service_unavailable' };
  }
  if (response.status === 400 || response.status === 401 || data.error === 'invalid_key') {
    return { ok: false, reason: 'invalid_key' };
  }
  return { ok: false, reason: 'unknown' };
}

export function HeroSection() {
  const [open, setOpen] = useState(false);
  const [accessKey, setAccessKey] = useState('');
  const [status, setStatus] = useState<AccessStatus>('idle');
  const [errorMessage, setErrorMessage] = useState('');
  const [installCommand, setInstallCommand] = useState<string | null>(null);

  const installUnlocked = Boolean(installCommand);

  const scrollToWaitlist = () => {
    setOpen(false);
    const waitlistSection = document.getElementById('waitlist');
    if (waitlistSection) {
      waitlistSection.scrollIntoView({ behavior: 'smooth' });
      const input = waitlistSection.querySelector('input');
      if (input) setTimeout(() => input.focus(), 500);
    }
  };

  const handleAccessSubmit = async (e: FormEvent) => {
    e.preventDefault();

    const trimmedKey = accessKey.trim();
    if (!trimmedKey || status === 'loading') return;

    setStatus('loading');
    setErrorMessage('');

    try {
      const result = await unlockInstallCommand(trimmedKey);
      if (!result.ok) {
        setStatus('error');
        setErrorMessage(
          result.reason === 'service_unavailable'
            ? 'Access service is temporarily unavailable. Try again in a moment.'
            : 'Invalid or expired early-access key'
        );
        return;
      }

      setInstallCommand(result.command);
      setStatus('success');
    } catch {
      setStatus('error');
      setErrorMessage('Could not reach the access service. Try again.');
    }
  };

  return (
    <section className="lg:min-h-screen pt-14">
      <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12 sm:py-16 lg:py-24 font-mono">
        <div className="grid gap-10 md:grid-cols-2 lg:gap-12 items-center">
          {/* Text Content - Left */}
          <div className="space-y-6 sm:space-y-8">
            {/* Product Identity */}
            <div className="flex items-center gap-3 sm:gap-4">
              <img
                src="/images/anvil-brandmark-ember.svg"
                alt=""
                aria-hidden="true"
                width={40}
                height={40}
                className="sm:w-12 sm:h-12"
              />
              <span className="font-mono text-lg sm:text-xl tracking-[0.2em] sm:tracking-[0.3em] text-anvil">
                anvil
              </span>
            </div>

            {/* Comment Line */}
            <div className="font-mono text-xs sm:text-sm text-text-muted tracking-wider">
              {'// SHIP_AT_AI_SPEED'}
            </div>

            <h1 className="font-mono text-2xl sm:text-4xl lg:text-5xl font-medium uppercase leading-tight tracking-tight text-text-primary text-balance">
              FORCE PROBABILISTIC TOOLS TO
              <br />
              <span className="text-anvil">RESPECT DETERMINISTIC RULES.</span>
            </h1>

            <p className="font-sans text-base sm:text-lg leading-relaxed text-text-muted max-w-lg">
              anvil enforces policy at generation time, not at review.
            </p>

            {/* Primary CTA - Install Terminal Box */}
            <div className="flex flex-col gap-3 sm:gap-4">
              <Dialog.Root open={open} onOpenChange={setOpen}>
                <Dialog.Trigger asChild>
                  <button className="w-full max-w-[450px] border border-anvil bg-anvil/5 px-4 sm:px-6 py-4 font-mono text-xs sm:text-sm text-left transition-colors hover:bg-anvil/10 cursor-pointer">
                    <span className="text-anvil">
                      $ {installCommand ?? REDACTED_INSTALL_COMMAND}
                    </span>
                    <span className="text-text-muted ml-4">
                      # {installUnlocked ? 'unlocked' : 'auth-required'}
                    </span>
                  </button>
                </Dialog.Trigger>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 bg-void/90 z-50" />
                  <Dialog.Content className="fixed inset-0 z-50 flex items-center justify-center p-4">
                    <div className="bg-surface border border-structure max-w-lg w-full p-6 space-y-6 font-mono">
                      <div className="flex items-start justify-between gap-4">
                        <div className="space-y-2">
                          <Dialog.Title className="text-lg sm:text-xl text-anvil uppercase tracking-tight">
                            Early-access gate
                          </Dialog.Title>
                          <Dialog.Description className="text-xs sm:text-sm text-text-muted leading-relaxed">
                            Enter your early-access key to reveal the install address.
                          </Dialog.Description>
                        </div>
                        <Dialog.Close asChild>
                          <button
                            type="button"
                            aria-label="Close"
                            className="text-text-muted transition-colors hover:text-text-primary"
                          >
                            [x]
                          </button>
                        </Dialog.Close>
                      </div>

                      <div className="border border-structure bg-void px-3 py-3 text-xs sm:text-sm">
                        <span className="text-text-muted">$ </span>
                        <span className={installUnlocked ? 'text-anvil' : 'text-text-muted'}>
                          {installCommand ?? REDACTED_INSTALL_COMMAND}
                        </span>
                      </div>

                      {installUnlocked ? (
                        <div className="space-y-4">
                          <p className="text-sm text-edda">[ OK ] access verified</p>
                          <p className="text-xs text-text-muted">
                            Homebrew install is unlocked for this session.
                          </p>
                          <Dialog.Close className="border border-edda px-4 py-2 text-xs sm:text-sm text-edda transition-colors hover:bg-edda/10 uppercase tracking-wide">
                            Close
                          </Dialog.Close>
                        </div>
                      ) : (
                        <form onSubmit={handleAccessSubmit} className="space-y-4">
                          <div className="space-y-2">
                            <label
                              htmlFor="early-access-key"
                              className="block text-xs text-text-muted"
                            >
                              Early-access key:
                            </label>
                            <input
                              id="early-access-key"
                              type="password"
                              value={accessKey}
                              onChange={(e) => setAccessKey(e.target.value)}
                              disabled={status === 'loading'}
                              placeholder="anvil_beta_..."
                              autoComplete="off"
                              className="w-full border border-structure bg-void px-3 py-2 font-mono text-sm text-text-primary placeholder:text-text-muted/40 focus:border-anvil focus:outline-none disabled:opacity-50"
                            />
                          </div>

                          {status === 'error' && (
                            <p className="text-xs text-anvil" role="alert">
                              {errorMessage}
                            </p>
                          )}

                          <div className="flex flex-col sm:flex-row gap-3 pt-2">
                            <button
                              type="submit"
                              disabled={status === 'loading'}
                              className="flex-1 border border-anvil bg-anvil/5 px-4 py-3 text-xs sm:text-sm text-anvil hover:bg-anvil/10 transition-colors uppercase tracking-wide disabled:opacity-50"
                            >
                              {status === 'loading' ? 'Checking...' : 'Unlock Install'}
                            </button>
                            <button
                              type="button"
                              onClick={scrollToWaitlist}
                              className="flex-1 border border-structure px-4 py-3 text-xs sm:text-sm text-text-muted hover:text-text-primary hover:border-text-muted transition-colors uppercase tracking-wide"
                            >
                              Request Access
                            </button>
                          </div>
                        </form>
                      )}
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>

              {/* Secondary - Docs */}
              <Link
                href="https://docs.eddacraft.ai/anvil/overview"
                className="block w-full max-w-[450px] border border-structure bg-transparent px-4 sm:px-6 py-3 font-mono text-xs sm:text-sm text-text-muted transition-colors hover:border-text-muted hover:text-text-primary text-left"
              >
                READ THE DOCS
              </Link>
            </div>

            {/* Stats */}
            <div className="flex gap-4 sm:gap-8 pt-4 border-t border-structure">
              <div>
                <div className="font-mono text-xl sm:text-2xl text-text-primary">10µs</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  save-time check
                </div>
              </div>
              <div>
                <div className="font-mono text-xl sm:text-2xl text-text-primary">800ns</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  policy eval
                </div>
              </div>
              <div>
                <div className="font-mono text-xl sm:text-2xl text-edda">14.5ms</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  cold graph build
                </div>
              </div>
            </div>
          </div>

          {/* Terminal - Right (hidden on mobile) */}
          <div className="hidden md:block lg:pl-8">
            <TerminalWindow />
          </div>
        </div>
      </div>
    </section>
  );
}
