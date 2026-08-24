const PARTS = [
  {
    title: 'GRAPH',
    state: 'operating foundation',
    current: true,
    text: 'living model of the software — structure, dependencies, symbols. context for protection and for the assistant.',
  },
  {
    title: 'POLICY ENGINE',
    state: 'operating foundation',
    current: true,
    text: 'deterministic rules at the control point. pass or fail, predictably. not the model that wrote the change.',
  },
  {
    title: 'LEARNING SYSTEM',
    state: 'being built',
    current: false,
    text: 'outcomes return to understanding. not a closed flywheel today.',
  },
] as const;

export function ProductStages() {
  return (
    <section className="site-section">
      <div className="site-container py-16 lg:py-20">
        <div className="mb-10 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="section-label mb-4">{'// PARTS'}</p>
            <h2 className="font-mono text-2xl uppercase text-off-white sm:text-3xl">
              THREE PARTS.
              <span className="mt-2 block text-anvil">ONE JUDGE.</span>
            </h2>
          </div>
          <p className="max-w-md font-sans text-sm leading-6 text-ghost-grey">
            Modes are understand, build, decide, learn. Parts are the graph, the policy engine, and
            the learning system. They are not the same list. Context supports humans and agents.
            anvil remains the independent control point, not the coding agent.
          </p>
        </div>

        <ol className="grid gap-px border border-structure bg-structure lg:grid-cols-3">
          {PARTS.map((part) => (
            <li key={part.title} className="bg-void p-5 sm:p-6">
              <h3 className="font-mono text-lg text-off-white">{part.title}</h3>
              <div
                className={`mt-4 font-mono text-[10px] uppercase tracking-wider ${
                  part.current ? 'text-edda' : 'text-ghost-grey'
                }`}
              >
                {part.state}
              </div>
              <p className="mt-4 font-sans text-sm leading-6 text-ghost-grey">{part.text}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
