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
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <BoundaryColumn label="operating today" items={OPERATING} current />
          <BoundaryColumn label="system being completed" items={COMPLETING} current={false} />
        </div>
      </div>
    </section>
  );
}
