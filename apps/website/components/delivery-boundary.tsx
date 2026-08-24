const OPERATING = [
  'deterministic pre-write and save-time protection',
  'resident graph and assistant-facing context',
  'checks, policy and configurable enforcement',
  'protection claims, witness chains and review capsules',
];

const COMPLETING = [
  'general intent conformance',
  'connected evidence providers',
  'independently verifiable decision receipts',
  'closed outcome-learning loop',
];

const DEFAULT_CHECKS = [
  ['secret-detection', 'credentials and secrets never enter the diff'],
  ['command-safety', 'destructive or unexpected shell is blocked'],
  ['antipattern-scan', 'known unsafe shapes in the proposed write'],
  ['import-boundaries', 'layer and package rules held at save-time'],
  ['team policy', 'deterministic rules as the organisation matures'],
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
          <span className="font-mono text-sm text-anvil">[ = ]</span>
          <h2 className="mt-5 font-mono text-3xl uppercase leading-tight text-off-white sm:text-4xl">
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
            useful at zero config. The no arrives at write-time, so the agent can correct.
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
