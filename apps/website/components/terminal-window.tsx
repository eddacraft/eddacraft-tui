'use client';

import { useEffect, useState } from 'react';

const COMMAND = 'anvil_validate_write  src/secret.ts';

export function TerminalWindow() {
  const [typedCommand, setTypedCommand] = useState('');
  const [showOutput, setShowOutput] = useState(false);

  useEffect(() => {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      const reducedMotionTimeout = window.setTimeout(() => {
        setTypedCommand(COMMAND);
        setShowOutput(true);
      }, 0);
      return () => window.clearTimeout(reducedMotionTimeout);
    }

    let charIndex = 0;
    let outputTimeout: number | undefined;
    const typeInterval = window.setInterval(() => {
      charIndex += 1;
      setTypedCommand(COMMAND.slice(0, charIndex));
      if (charIndex >= COMMAND.length) {
        window.clearInterval(typeInterval);
        outputTimeout = window.setTimeout(() => setShowOutput(true), 300);
      }
    }, 42);

    return () => {
      window.clearInterval(typeInterval);
      if (outputTimeout) window.clearTimeout(outputTimeout);
    };
  }, []);

  return (
    <div className="w-full border border-structure bg-surface font-mono text-[11px] leading-relaxed sm:text-xs">
      <div className="flex items-center justify-between border-b border-structure px-4 py-2 text-ghost-grey">
        <span>anvil :: pre-write</span>
        <span>~/project</span>
      </div>

      <div className="min-h-[28rem] space-y-4 p-4 sm:p-5">
        <div className="flex min-w-0 items-center gap-2">
          <span className="text-anvil">$</span>
          <span className="break-all text-off-white">{typedCommand}</span>
          {!showOutput ? <span className="cursor-blink text-anvil">▊</span> : null}
        </div>

        {showOutput ? (
          <div className="space-y-4" aria-live="polite">
            <div className="text-anvil">[ = ] PRE_WRITE_VALIDATION</div>

            <dl className="grid grid-cols-[7rem_1fr] gap-x-3 text-ghost-grey">
              <dt>path</dt>
              <dd className="min-w-0 break-all text-off-white">src/secret.ts</dd>
              <dt>backend</dt>
              <dd className="text-off-white">daemon</dd>
              <dt>mode</dt>
              <dd className="text-off-white">preWrite</dd>
              <dt>enforcement</dt>
              <dd className="text-off-white">interrupt</dd>
            </dl>

            <div className="border-y border-structure py-3">
              <div className="flex gap-3 text-brick-red">
                <span>[ ERR ]</span>
                <span>secret-detection</span>
              </div>
              <p className="mt-2 pl-[4.75rem] text-ghost-grey">
                credential-like token in proposed content
              </p>
            </div>

            <dl className="grid grid-cols-[7rem_1fr] gap-x-3">
              <dt className="text-ghost-grey">decision</dt>
              <dd className="text-brick-red">interrupt</dd>
              <dt className="text-ghost-grey">safe_default</dt>
              <dd className="text-off-white">do-not-write</dd>
            </dl>

            <div className="border-t border-structure pt-3">
              <div className="text-anvil">[ protection_claim ]</div>
              <div className="mt-2 grid grid-cols-[7rem_1fr] gap-x-3">
                <span className="text-ghost-grey">schema</span>
                <span className="break-all text-off-white">anvil.protection-claim.v1</span>
                <span className="text-ghost-grey">worktree</span>
                <span className="text-edda">pre-write-daemon</span>
              </div>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
