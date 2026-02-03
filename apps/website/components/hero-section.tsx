'use client';

import { useState } from 'react';
import { TerminalWindow } from './terminal-window';

export function HeroSection() {
  const [showDocsModal, setShowDocsModal] = useState(false);

  return (
    <section className="lg:min-h-screen pt-14">
      <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12 sm:py-16 lg:py-24 font-mono">
        <div className="grid gap-10 lg:grid-cols-2 lg:gap-12 items-center">
          {/* Text Content - Left */}
          <div className="space-y-6 sm:space-y-8">
            {/* Product Identity */}
            <div className="flex items-center gap-3 sm:gap-4">
              <img
                src="/images/anvil-brandmark-ember.svg"
                alt="Anvil"
                width={40}
                height={40}
                className="sm:w-12 sm:h-12"
              />
              <span className="font-mono text-lg sm:text-xl uppercase tracking-[0.2em] sm:tracking-[0.3em] text-anvil">
                Anvil
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
              Anvil enforces policy at generation time, not at review.
            </p>

            {/* Primary CTA - NPM Terminal Box */}
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
                <span className="text-anvil">$ npm i -g @eddacraft/anvil</span>
                <span className="text-text-muted ml-4"># closed-beta</span>
              </button>

              {/* Secondary - Docs */}
              <button
                onClick={() => setShowDocsModal(true)}
                className="w-full max-w-[450px] border border-structure bg-transparent px-4 sm:px-6 py-3 font-mono text-xs sm:text-sm text-text-muted transition-colors hover:border-text-muted hover:text-text-primary text-left"
              >
                READ THE DOCS
              </button>
            </div>

            {/* Stats */}
            <div className="flex gap-4 sm:gap-8 pt-4 border-t border-structure">
              <div>
                <div className="font-mono text-xl sm:text-2xl text-text-primary">{'<'}50ms</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  gate latency
                </div>
              </div>
              <div>
                <div className="font-mono text-xl sm:text-2xl text-text-primary">100%</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  deterministic
                </div>
              </div>
              <div>
                <div className="font-mono text-xl sm:text-2xl text-edda">0</div>
                <div className="font-mono text-[10px] sm:text-xs text-text-muted uppercase tracking-wider">
                  config drift
                </div>
              </div>
            </div>
          </div>

          {/* Terminal - Right (hidden on mobile) */}
          <div className="hidden lg:block lg:pl-8">
            <TerminalWindow />
          </div>
        </div>
      </div>

      {/* Docs Coming Soon Modal */}
      {showDocsModal && (
        <div
          className="fixed inset-0 bg-void/90 flex items-center justify-center z-50 p-4"
          onClick={() => setShowDocsModal(false)}
        >
          <div
            className="bg-surface border border-structure max-w-md w-full p-6 space-y-6 font-mono"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Modal Header */}
            <div className="text-xs text-text-muted uppercase tracking-wide">
              {'// DOCUMENTATION_STATUS'}
            </div>

            {/* Modal Content */}
            <div className="space-y-4">
              <div className="text-sm text-text-muted">
                <span className="text-anvil">$</span> man anvil
              </div>
              <div className="border-l-2 border-anvil pl-4 py-2">
                <p className="text-sm text-text-primary">No manual entry for anvil</p>
                <p className="text-xs text-text-muted mt-2">
                  Documentation is being compiled. <br />
                  Expected availability: pre-release cohort.
                </p>
              </div>
            </div>

            {/* Modal Actions */}
            <div className="flex flex-col sm:flex-row gap-3 pt-2">
              <button
                onClick={() => {
                  setShowDocsModal(false);
                  const waitlistSection = document.getElementById('waitlist');
                  if (waitlistSection) {
                    waitlistSection.scrollIntoView({ behavior: 'smooth' });
                    const input = waitlistSection.querySelector('input');
                    if (input) setTimeout(() => input.focus(), 500);
                  }
                }}
                className="flex-1 border border-anvil bg-anvil/5 px-4 py-3 text-xs sm:text-sm text-anvil hover:bg-anvil/10 transition-colors uppercase tracking-wide"
              >
                Request Access
              </button>
              <button
                onClick={() => setShowDocsModal(false)}
                className="flex-1 border border-structure px-4 py-3 text-xs sm:text-sm text-text-muted hover:text-text-primary hover:border-text-muted transition-colors uppercase tracking-wide"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
