'use client';

import { useState, type FormEvent } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import Link from 'next/link';
import { scrollToWaitlist as scrollToWaitlistSection } from '@/lib/scroll';
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
    scrollToWaitlistSection();
  };

  const handleAccessSubmit = async (event: FormEvent) => {
    event.preventDefault();

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
    <section id="product" className="pt-14">
      <div className="site-container grid gap-12 py-16 md:grid-cols-[0.9fr_1.1fr] md:items-center lg:gap-20 lg:py-24">
        <div>
          <div className="mb-8 flex items-center gap-3 font-mono text-sm text-anvil">
            <img
              src="/images/anvil-brandmark-ember.svg"
              alt=""
              aria-hidden="true"
              width={28}
              height={28}
            />
            <span className="tracking-[0.2em]">anvil</span>
          </div>

          <p className="section-label mb-5">{'// GENERATION_TIME_TRUST'}</p>
          <h1 className="max-w-2xl font-mono text-4xl font-medium uppercase leading-[1.08] tracking-[-0.04em] text-off-white sm:text-5xl lg:text-6xl">
            TRUST THE CODE
            <br />
            <span className="text-anvil">YOUR AI WRITES.</span>
          </h1>

          <p className="mt-7 max-w-xl font-sans text-lg leading-8 text-off-white">
            anvil is the independent, deterministic control point for AI-assisted software
            engineering.
          </p>
          <p className="mt-5 max-w-xl font-sans text-base leading-7 text-ghost-grey">
            Understand the change. Apply your standards. Stop unsafe work before it reaches review.
          </p>

          <div className="mt-9 flex max-w-xl flex-col gap-3 sm:flex-row">
            <button
              type="button"
              onClick={scrollToWaitlist}
              className="border border-anvil bg-anvil px-5 py-3 font-mono text-xs uppercase tracking-wide text-void transition-colors hover:bg-anvil/90"
            >
              [ = ] request early access
            </button>
            <Link
              href="https://docs.eddacraft.ai/anvil/overview"
              className="border border-structure px-5 py-3 font-mono text-xs uppercase tracking-wide text-ghost-grey transition-colors hover:border-border-strong hover:text-off-white"
            >
              read the docs
            </Link>
          </div>

          <Dialog.Root open={open} onOpenChange={setOpen}>
            <Dialog.Trigger asChild>
              <button
                type="button"
                className="mt-4 block w-full max-w-xl border border-structure bg-surface px-4 py-3 text-left font-mono text-xs transition-colors hover:border-border-strong"
              >
                <span className="text-anvil">$ {installCommand ?? REDACTED_INSTALL_COMMAND}</span>
                <span className="ml-3 text-ghost-grey">
                  # {installUnlocked ? 'unlocked' : 'auth-required'}
                </span>
              </button>
            </Dialog.Trigger>
            <Dialog.Portal>
              <Dialog.Overlay className="fixed inset-0 z-50 bg-void/90" />
              <Dialog.Content className="fixed inset-0 z-50 flex items-center justify-center p-4">
                <div className="w-full max-w-lg space-y-6 border border-structure bg-surface p-6 font-mono">
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-2">
                      <Dialog.Title className="text-lg uppercase text-anvil">
                        Early-access gate
                      </Dialog.Title>
                      <Dialog.Description className="text-xs leading-relaxed text-ghost-grey sm:text-sm">
                        Enter your early-access key to reveal the install address.
                      </Dialog.Description>
                    </div>
                    <Dialog.Close asChild>
                      <button type="button" aria-label="Close" className="text-ghost-grey">
                        [x]
                      </button>
                    </Dialog.Close>
                  </div>

                  <div className="border border-structure bg-void px-3 py-3 text-xs sm:text-sm">
                    <span className="text-ghost-grey">$ </span>
                    <span className={installUnlocked ? 'text-anvil' : 'text-ghost-grey'}>
                      {installCommand ?? REDACTED_INSTALL_COMMAND}
                    </span>
                  </div>

                  {installUnlocked ? (
                    <div className="space-y-4">
                      <p className="text-sm text-edda">[ OK ] access verified</p>
                      <p className="text-xs text-ghost-grey">
                        Homebrew install is unlocked for this session.
                      </p>
                      <Dialog.Close className="border border-edda px-4 py-2 text-xs uppercase tracking-wide text-edda transition-colors hover:bg-edda/10">
                        Close
                      </Dialog.Close>
                    </div>
                  ) : (
                    <form onSubmit={handleAccessSubmit} className="space-y-4">
                      <div className="space-y-2">
                        <label htmlFor="early-access-key" className="block text-xs text-ghost-grey">
                          Early-access key:
                        </label>
                        <input
                          id="early-access-key"
                          type="password"
                          value={accessKey}
                          onChange={(event) => setAccessKey(event.target.value)}
                          disabled={status === 'loading'}
                          placeholder="anvil_beta_..."
                          autoComplete="off"
                          className="w-full border border-structure bg-void px-3 py-2 font-mono text-sm text-off-white placeholder:text-ghost-grey/50 focus:border-anvil focus:outline-none disabled:opacity-50"
                        />
                      </div>

                      {status === 'error' ? (
                        <p className="text-xs text-brick-red" role="alert">
                          {errorMessage}
                        </p>
                      ) : null}

                      <div className="flex flex-col gap-3 pt-2 sm:flex-row">
                        <button
                          type="submit"
                          disabled={status === 'loading'}
                          className="flex-1 border border-anvil bg-anvil/5 px-4 py-3 text-xs uppercase tracking-wide text-anvil transition-colors hover:bg-anvil/10 disabled:opacity-50"
                        >
                          {status === 'loading' ? 'Checking...' : 'Unlock install'}
                        </button>
                        <button
                          type="button"
                          onClick={scrollToWaitlist}
                          className="flex-1 border border-structure px-4 py-3 text-xs uppercase tracking-wide text-ghost-grey transition-colors hover:border-border-strong hover:text-off-white"
                        >
                          Request access
                        </button>
                      </div>
                    </form>
                  )}
                </div>
              </Dialog.Content>
            </Dialog.Portal>
          </Dialog.Root>
        </div>

        <TerminalWindow />
      </div>
    </section>
  );
}
