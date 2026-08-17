const STAGES = [
  {
    title: 'UNDERSTAND',
    current: 'Resident graph, software structure, dependencies and symbols.',
    direction: 'General intent context.',
  },
  {
    title: 'BUILD',
    current: 'Proposed-change context, graph queries and impact analysis.',
    direction: 'Connected evidence assembly.',
  },
  {
    title: 'DECIDE',
    current: 'Pre-write validation, deterministic policy and enforcement modes.',
    direction: 'Durable decision receipts.',
  },
  {
    title: 'LEARN',
    current: 'Witness chains, drift snapshots and review capsules.',
    direction: 'Closed outcome learning.',
  },
] as const;

export function ProductStages() {
  return (
    <section className="site-section">
      <div className="site-container py-16 lg:py-20">
        <div className="mb-10 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="section-label mb-4">{'// FOUR_STAGE_CONTROL_PLANE'}</p>
            <h2 className="font-mono text-2xl uppercase text-off-white sm:text-3xl">
              anvil product architecture
            </h2>
          </div>
          <p className="max-w-md font-sans text-sm leading-6 text-ghost-grey">
            Context supports humans and agents. anvil remains the independent control point, not the
            coding agent.
          </p>
        </div>

        <ol className="grid gap-px border border-structure bg-structure md:grid-cols-2 xl:grid-cols-4">
          {STAGES.map((stage, index) => (
            <li key={stage.title} className="bg-void p-5 sm:p-6">
              <div className="font-mono text-sm text-anvil">[ {index + 1} ]</div>
              <h3 className="mt-5 font-mono text-lg text-off-white">{stage.title}</h3>
              <div className="mt-5 space-y-4 text-sm leading-6">
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-wider text-edda">
                    operating foundation
                  </div>
                  <p className="mt-1 font-sans text-ghost-grey">{stage.current}</p>
                </div>
                <div>
                  <div className="font-mono text-[10px] uppercase tracking-wider text-ghost-grey">
                    direction
                  </div>
                  <p className="mt-1 font-sans text-ghost-grey/70">{stage.direction}</p>
                </div>
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
