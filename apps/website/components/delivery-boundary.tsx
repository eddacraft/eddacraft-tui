const OPERATING = [
  'deterministic checks before a write and at save-time',
  'a resident graph of the code, with context served to your assistant',
  'built-in checks, your own policy, and enforcement you choose',
  'a durable record of what was checked, for review and audit',
];

const COMPLETING = [
  'checking a change against the intent behind it',
  'evidence drawn from the tools you already run',
  'decision receipts a third party can verify',
  'outcomes feeding back into future decisions',
];

const DEFAULT_CHECKS = [
  ['secret-detection', 'credentials and tokens never enter the diff'],
  ['command-safety', 'destructive or unexpected shell is stopped'],
  ['antipattern-scan', 'known unsafe shapes caught in the proposed write'],
  ['import-boundaries', 'layer and package rules held at save-time'],
  ['your policy', 'your own rules, evaluated the same way every time'],
] as const;

function BoundaryColumn({
  label,
  items,
  current,
}: {
  label: string;
  items: readonly string[];
  current: boolean;
}) {
  return (
    <div className="border border-structure bg-void p-5 sm:p-6">
      <h3
        className={`font-mono text-xs uppercase tracking-wider ${current ? 'text-edda' : 'text-ghost-grey'}`}
      >
        {label}
      </h3>
      <ul className="mt-5 space-y-3 font-sans text-sm leading-6 text-ghost-grey">
        {items.map((item) => (
          <li key={item} className="flex gap-3">
            <span aria-hidden="true" className={current ? 'text-edda' : 'text-ghost-grey'}>
              {current ? '[ = ]' : '[ ]'}
            </span>
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function DeliveryBoundary() {
  return (
    <section id="roadmap" className="site-section bg-surface">
      <div className="site-container grid gap-10 py-16 lg:grid-cols-[0.8fr_1.2fr] lg:items-start lg:py-20">
        <div>
          <p className="section-label mb-5">{'// DELIVERY_BOUNDARY'}</p>
          <h2 className="font-mono text-3xl uppercase leading-tight text-off-white sm:text-4xl">
            THE CONTROL POINT
            <br />
            <span className="text-anvil">SHIPS TODAY.</span>
            <br />
            THE TRUST CHAIN
            <br />
            <span className="text-ghost-grey">COMES NEXT.</span>
          </h2>
          <p className="mt-6 max-w-md font-sans text-sm leading-6 text-ghost-grey">
            anvil sits in the workflow as an independent, deterministic judge. The first run is
            useful with zero configuration. A refusal arrives at write-time, while the agent can
            still correct the work.
          </p>
          <p className="mt-4 max-w-md font-sans text-sm leading-6 text-ghost-grey">
            This is the one place on this page where the line between shipped and planned is drawn.
          </p>
        </div>
        <div className="space-y-4">
          <div className="border border-structure bg-void">
            <p className="border-b border-structure px-5 py-3 font-mono text-xs uppercase tracking-wider text-ghost-grey">
              default checks
            </p>
            <ul>
              {DEFAULT_CHECKS.map(([name, text]) => (
                <li
                  key={name}
                  className="flex flex-col gap-1 border-b border-structure px-5 py-3 last:border-b-0 sm:flex-row sm:items-baseline sm:justify-between sm:gap-6"
                >
                  <span className="font-mono text-sm text-off-white">{name}</span>
                  <span className="font-sans text-sm text-ghost-grey sm:text-right">{text}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <BoundaryColumn label="operating today" items={OPERATING} current />
            <BoundaryColumn label="system being completed" items={COMPLETING} current={false} />
          </div>
        </div>
      </div>
    </section>
  );
}
