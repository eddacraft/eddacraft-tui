'use client';

import * as Dialog from '@radix-ui/react-dialog';
import Link from 'next/link';

export function Navbar() {
  const scrollToWaitlist = () => {
    const waitlistSection = document.getElementById('waitlist');
    if (waitlistSection) {
      waitlistSection.scrollIntoView({ behavior: 'smooth' });
      const input = waitlistSection.querySelector('input');
      if (input) setTimeout(() => input.focus(), 500);
    }
  };

  return (
    <>
      <nav className="fixed top-0 left-0 right-0 z-50 h-14 border-b border-structure bg-void">
        <div className="mx-auto flex h-full max-w-6xl items-center justify-between px-4 sm:px-6">
          <Link
            href="/"
            className="flex items-center gap-2 font-mono text-sm text-text-primary shrink-0"
          >
            <img
              src="/images/eddacraft-brandmark-white.svg"
              alt="eddacraft"
              width={18}
              height={18}
            />
            <span className="hidden sm:inline">eddacraft</span>
          </Link>

          <div className="flex items-center gap-4 sm:gap-8 font-mono text-xs sm:text-sm">
            <Link
              href="https://docs.eddacraft.ai/anvil/overview"
              className="text-anvil transition-colors hover:text-text-primary"
            >
              Anvil
            </Link>

            {/* Edda - coming soon dialog */}
            <Dialog.Root>
              <Dialog.Trigger asChild>
                <button className="text-text-muted transition-colors hover:text-text-primary hidden sm:inline cursor-pointer bg-transparent border-none font-mono text-xs sm:text-sm p-0">
                  Edda
                </button>
              </Dialog.Trigger>
              <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-void/90 z-50" />
                <Dialog.Content className="fixed inset-0 z-50 flex items-center justify-center p-4">
                  <div className="bg-surface border border-structure max-w-md w-full p-6 space-y-6 font-mono">
                    <Dialog.Title className="text-lg sm:text-xl text-edda uppercase tracking-tight">
                      Edda is coming soon
                    </Dialog.Title>
                    <Dialog.Description className="text-sm text-text-muted leading-relaxed">
                      The full Edda stack is under active development. Request access to be notified
                      when it becomes available.
                    </Dialog.Description>
                    <div className="flex flex-col sm:flex-row gap-3 pt-2">
                      <Dialog.Close asChild>
                        <button
                          onClick={scrollToWaitlist}
                          className="flex-1 border border-edda bg-edda/5 px-4 py-3 text-xs sm:text-sm text-edda hover:bg-edda/10 transition-colors uppercase tracking-wide"
                        >
                          Request Access
                        </button>
                      </Dialog.Close>
                      <Dialog.Close asChild>
                        <button className="flex-1 border border-structure px-4 py-3 text-xs sm:text-sm text-text-muted hover:text-text-primary hover:border-text-muted transition-colors uppercase tracking-wide">
                          Close
                        </button>
                      </Dialog.Close>
                    </div>
                  </div>
                </Dialog.Content>
              </Dialog.Portal>
            </Dialog.Root>

            <Link
              href="https://docs.eddacraft.ai"
              className="text-text-muted transition-colors hover:text-text-primary"
            >
              Docs
            </Link>

            {/* Login - pre-release dialog */}
            <Dialog.Root>
              <Dialog.Trigger asChild>
                <button className="text-text-muted transition-colors hover:text-text-primary cursor-pointer bg-transparent border-none font-mono text-xs sm:text-sm p-0">
                  Login
                </button>
              </Dialog.Trigger>
              <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-void/90 z-50" />
                <Dialog.Content className="fixed inset-0 z-50 flex items-center justify-center p-4">
                  <div className="bg-surface border border-structure max-w-md w-full p-6 space-y-6 font-mono">
                    <Dialog.Title className="text-lg sm:text-xl text-anvil uppercase tracking-tight">
                      Login is not yet available
                    </Dialog.Title>
                    <Dialog.Description asChild>
                      <div className="space-y-4">
                        <div className="text-sm text-text-muted">
                          <span className="text-anvil">$</span> anvil login
                        </div>
                        <div className="border-l-2 border-anvil pl-4 py-2">
                          <p className="text-xs text-text-muted">
                            Anvil is in pre-release. Request access to join the next available
                            cohort.
                          </p>
                        </div>
                      </div>
                    </Dialog.Description>
                    <div className="flex flex-col sm:flex-row gap-3 pt-2">
                      <Dialog.Close asChild>
                        <button
                          onClick={scrollToWaitlist}
                          className="flex-1 border border-anvil bg-anvil/5 px-4 py-3 text-xs sm:text-sm text-anvil hover:bg-anvil/10 transition-colors uppercase tracking-wide"
                        >
                          Request Access
                        </button>
                      </Dialog.Close>
                      <Dialog.Close asChild>
                        <button className="flex-1 border border-structure px-4 py-3 text-xs sm:text-sm text-text-muted hover:text-text-primary hover:border-text-muted transition-colors uppercase tracking-wide">
                          Close
                        </button>
                      </Dialog.Close>
                    </div>
                  </div>
                </Dialog.Content>
              </Dialog.Portal>
            </Dialog.Root>
          </div>
        </div>
      </nav>
    </>
  );
}
