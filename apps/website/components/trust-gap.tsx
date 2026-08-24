const IMPACTS = [
  ['REVIEW_CAPACITY', 'output outruns the people responsible for the system'],
  ['SELF_REPORT', 'the system that created the work cannot judge it'],
  ['SILENT_HOUR', 'non-conforming writes land before anyone looks'],
] as const;

const AUTONOMY = [
  ['BABYSIT', 'you keep up. the agent does not.'],
  ['UNSUPERVISED', 'the agent keeps up. you do not.'],
  ['ANVIL', 'the agent runs. the write still has an independent judge.'],
] as const;

export function TrustGap() {
  return (
    <>
      <section id="why" className="site-section">
        <div className="site-container grid gap-10 py-16 md:grid-cols-[0.8fr_1.2fr] lg:py-20">
          <div>
            <p className="section-label mb-5">{'// THE_TRUST_GAP'}</p>
            <h2 className="max-w-lg font-mono text-3xl uppercase leading-tight tracking-tight text-off-white sm:text-4xl">
              AI CAN CREATE MORE
              <br />
              <span className="text-anvil">THAN HUMANS CAN REVIEW.</span>
            </h2>
          </div>
          <div className="space-y-6 font-sans text-base leading-7 text-ghost-grey">
            <p>
              AI increases how much software an organisation can produce. It also increases the
              distance between the people responsible for a system and the actions taken on their
              behalf.
            </p>
            <p>
              After an unsupervised hour you cannot reconstruct what happened from the agent’s
              account. Asking the agent is not an audit: the model is biased toward its own work,
              compaction already dropped the rules file, and re-analysing raw logs is a tax.
            </p>
            <p>
              Logs, monitoring, governance programmes and model-provider explanations can each show
              something. None of the systems teams use today establishes, on its own, whether a
              particular AI-assisted change deserved to be trusted.
            </p>
            <dl className="grid gap-px border border-structure bg-structure sm:grid-cols-3">
              {IMPACTS.map(([term, meaning]) => (
                <div key={term} className="bg-void p-4">
                  <dt className="font-mono text-xs text-anvil">{term}</dt>
                  <dd className="mt-2 text-sm text-ghost-grey">{meaning}</dd>
                </div>
              ))}
            </dl>
            <div className="grid gap-px bg-structure md:grid-cols-3">
              {AUTONOMY.map(([label, text], index) => (
                <div
                  key={label}
                  className={`bg-void p-4 ${index === 2 ? 'border-l border-anvil' : ''}`}
                >
                  <p
                    className={`font-mono text-xs uppercase tracking-wider ${
                      index === 2 ? 'text-anvil' : 'text-ghost-grey'
                    }`}
                  >
                    {label}
                  </p>
                  <p className="mt-2 text-sm text-off-white">{text}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>

      <section className="site-section bg-surface">
        <div className="site-container grid gap-10 py-14 md:grid-cols-[1.2fr_0.8fr] md:items-center">
          <div>
            <span className="font-mono text-sm text-anvil">[ = ]</span>
            <h2 className="mt-5 font-mono text-2xl uppercase leading-snug tracking-tight text-off-white sm:text-3xl">
              PROTECTION IS THE ENTRY POINT.
              <br />
              <span className="text-anvil">DECISION INTEGRITY IS THE SYSTEM AROUND IT.</span>
            </h2>
          </div>
          <div className="space-y-4 font-sans text-base leading-7 text-ghost-grey">
            <p>
              Today, anvil protects supported software changes as they are written and maintains a
              living model of the code beneath them.
            </p>
            <p>
              That control point is the foundation for a broader system connecting intent, evidence,
              policy and durable decisions.
            </p>
          </div>
        </div>
      </section>
    </>
  );
}
