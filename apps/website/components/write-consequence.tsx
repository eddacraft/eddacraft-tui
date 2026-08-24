import type { ReactNode } from 'react';

function TerminalFrame({
  title,
  hint,
  tone,
  children,
}: {
  title: string;
  hint: string;
  tone: 'err' | 'ok';
  children: ReactNode;
}) {
  return (
    <div className="border border-structure bg-void font-mono text-sm">
      <div className="flex items-center justify-between border-b border-structure px-4 py-2 text-xs uppercase tracking-wider">
        <span className={tone === 'err' ? 'text-brick-red' : 'text-edda'}>{title}</span>
        <span className="text-ghost-grey">{hint}</span>
      </div>
      <div className="space-y-2 p-5 leading-6">{children}</div>
    </div>
  );
}

export function WriteConsequence() {
  return (
    <section className="site-section">
      <div className="site-container py-16 lg:py-20">
        <p className="section-label mb-5">{'// CONSEQUENCE'}</p>
        <h2 className="max-w-3xl font-mono text-3xl uppercase leading-tight text-off-white sm:text-4xl">
          THE WRITE EITHER HAS A JUDGE,
          <span className="mt-2 block text-anvil">OR IT DOES NOT.</span>
        </h2>
        <div className="mt-10 grid gap-px bg-structure lg:grid-cols-2">
          <TerminalFrame title="WITHOUT" hint="no judge" tone="err">
            <p className="text-ghost-grey">$ agent write src/auth.ts</p>
            <p className="text-off-white">skip CLAUDE.md</p>
            <p className="text-brick-red">secret lands in src/auth.ts</p>
            <p className="text-ghost-grey">the miss arrives at review, or in production</p>
          </TerminalFrame>
          <TerminalFrame title="WITH" hint="independent judge" tone="ok">
            <p className="text-ghost-grey">$ agent write src/auth.ts</p>
            <p className="text-brick-red">[ ERR ] secret-detection</p>
            <p className="text-edda">write blocked at save-time</p>
            <p className="text-ghost-grey">agent sees the miss immediately</p>
          </TerminalFrame>
        </div>
      </div>
    </section>
  );
}
