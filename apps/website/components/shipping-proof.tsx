const PROOF_POINTS = [
  'LOCAL EXECUTION',
  'DETERMINISTIC POLICY',
  'RESIDENT GRAPH',
  '12 MCP CLIENTS',
];

export function ShippingProof() {
  return (
    <section aria-label="Current product proof" className="site-container pb-10">
      <div className="border border-structure font-mono text-[10px] uppercase tracking-wider text-ghost-grey sm:text-xs">
        <div className="grid divide-y divide-structure sm:grid-cols-2 sm:divide-x sm:divide-y-0 lg:grid-cols-4">
          {PROOF_POINTS.map((point) => (
            <div key={point} className="flex items-center justify-between px-4 py-3">
              <span className="text-anvil">[ = ]</span>
              <span>{point}</span>
            </div>
          ))}
        </div>
        <p className="border-t border-structure px-4 py-2 text-right text-[9px] text-ghost-grey/80">
          reference: 28.3 µs incremental :: 1.6 µs policy :: deus :: 2026-06-26
        </p>
      </div>
    </section>
  );
}
