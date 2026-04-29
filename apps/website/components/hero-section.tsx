'use client';

import Link from 'next/link';
import { TerminalWindow } from './terminal-window';

export function HeroSection() {
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
              <button
                className="w-full max-w-[450px] border border-anvil bg-anvil/5 px-4 sm:px-6 py-4 font-mono text-xs sm:text-sm text-left transition-colors hover:bg-anvil/10 cursor-pointer"
                onClick={() => {
                  const waitlistSection = document.getElementById('waitlist');
                  if (waitlistSection) {
                    waitlistSection.scrollIntoView({ behavior: 'smooth' });
                    const input = waitlistSection.querySelector('input');
                    if (input) setTimeout(() => input.focus(), 500);
                  }
                }}
              >
                <span className="text-anvil">$ brew install eddacraft/tap/anvil</span>
                <span className="text-text-muted ml-4"># early-access</span>
              </button>

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
