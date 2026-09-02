const STAGES = [
  {
    title: 'UNDERSTAND',
    state: 'operating today',
    current: true,
    text: 'a resident graph of your software: structure, dependencies, symbols and ownership. it is the context for every check, and for your assistant.',
  },
  {
    title: 'BUILD',
    state: 'operating today',
    current: true,
    text: 'context, impact analysis and explanation for the humans and agents doing the work. anvil supports the change; the coding agent still writes it.',
  },
  {
    title: 'DECIDE',
    state: 'operating today',
    current: true,
    text: 'interception and deterministic policy at the control point. block or pass, the same way every time, by software that did not write the change.',
  },
  {
    title: 'LEARN',
    state: 'being built',
    current: false,
    text: 'drift against your baseline is checked today. outcomes feeding back into the next decision is the part still being built.',
  },
] as const;

export function ProductStages() {
  return (
    <section className="site-section">
      <div className="site-container py-16 lg:py-20">
        <div className="mb-10 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <p className="section-label mb-4">{'// FOUR_STAGES'}</p>
            <h2 className="font-mono text-2xl uppercase text-off-white sm:text-3xl">
              FOUR STAGES.
              <span className="mt-2 block text-anvil">ONE JUDGE.</span>
            </h2>
          </div>
          <p className="max-w-md font-sans text-sm leading-6 text-ghost-grey">
            Understand, build, decide, learn. anvil is the judge inside that loop. It informs the
            change and rules on it. It does not write it.
          </p>
        </div>

        <ol className="grid gap-px border border-structure bg-structure sm:grid-cols-2 lg:grid-cols-4">
          {STAGES.map((stage) => (
            <li key={stage.title} className="bg-void p-5 sm:p-6">
              <h3 className="font-mono text-lg text-off-white">{stage.title}</h3>
              <div
                className={`mt-4 font-mono text-[10px] uppercase tracking-wider ${
                  stage.current ? 'text-edda' : 'text-ghost-grey'
                }`}
              >
                {stage.state}
              </div>
              <p className="mt-4 font-sans text-sm leading-6 text-ghost-grey">{stage.text}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
