'use client';

import { useEffect, useState } from 'react';

export function TerminalWindow() {
  const [typedCommand, setTypedCommand] = useState('');
  const [showOutput, setShowOutput] = useState(false);
  const command = 'anvil gate plan.md';

  useEffect(() => {
    let charIndex = 0;
    const typeInterval = setInterval(() => {
      if (charIndex < command.length) {
        setTypedCommand(command.slice(0, charIndex + 1));
        charIndex++;
      } else {
        clearInterval(typeInterval);
        setTimeout(() => setShowOutput(true), 400);
      }
    }, 80);

    return () => clearInterval(typeInterval);
  }, []);

  return (
    <div className="w-full border border-structure bg-void font-mono text-xs sm:text-sm overflow-x-auto">
      {/* Terminal Header */}
      <div className="flex items-center gap-2 border-b border-structure px-3 sm:px-4 py-2">
        <span className="text-text-muted">anvil</span>
        <span className="text-structure">—</span>
        <span className="text-text-muted text-xs">~/project</span>
      </div>

      {/* Terminal Content */}
      <div className="p-3 sm:p-4 space-y-3 min-w-0">
        {/* Command Line */}
        <div className="flex items-center gap-2">
          <span className="text-anvil">$</span>
          <span className="text-text-primary">{typedCommand}</span>
          {!showOutput && <span className="cursor-blink text-anvil">▊</span>}
        </div>

        {/* Output */}
        {showOutput && (
          <div className="mt-4 space-y-3 sm:space-y-4 overflow-x-auto">
            {/* Header */}
            <div className="text-text-muted whitespace-nowrap">
              ╭──────────────────────────────╮
            </div>
            <div className="pl-2 text-text-muted whitespace-nowrap">
              │ <span className="text-text-primary">RISK_ANALYSIS</span> :: plan.md
            </div>
            <div className="text-text-muted whitespace-nowrap">
              ├──────────────────────────────┤
            </div>

            {/* Error */}
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-anvil">[ ERR ]</span>
              <span className="text-text-primary">POLICY_VIOLATION</span>
            </div>
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-text-muted pl-4 sm:pl-8">→ Unreviewed dependency: </span>
            </div>
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-anvil pl-6 sm:pl-10">lodash@4.17.21</span>
            </div>

            {/* Divider */}
            <div className="text-text-muted whitespace-nowrap">
              ├──────────────────────────────┤
            </div>

            {/* Success */}
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-edda">[ OK ]</span>
              <span className="text-text-primary">SECURITY_SCAN</span>
            </div>
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-text-muted pl-4 sm:pl-8">→ No secrets detected</span>
            </div>
            <div className="pl-2 flex items-start gap-2 whitespace-nowrap">
              <span className="text-text-muted">│</span>
              <span className="text-edda pl-6 sm:pl-10">0 vulnerabilities</span>
            </div>

            {/* Footer */}
            <div className="text-text-muted whitespace-nowrap">
              ╰──────────────────────────────╯
            </div>

            {/* Summary */}
            <div className="pt-2 flex items-center gap-2 sm:gap-4 text-xs flex-wrap">
              <span className="text-text-muted">gate:</span>
              <span className="text-anvil">BLOCKED</span>
              <span className="text-text-muted">|</span>
              <span className="text-text-muted">hash:</span>
              <span className="text-text-muted">a7f3e2d</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
