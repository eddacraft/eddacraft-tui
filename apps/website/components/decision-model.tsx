const MODEL_STEPS = [
  ['INTENT', 'what outcome was expected'],
  ['EVIDENCE', 'what was demonstrably true'],
  ['POLICY', 'which constraints applied'],
  ['DETERMINISTIC DECISION', 'the independent trust boundary'],
  ['DECISION RECEIPT', 'why the action was trusted'],
] as const;

export function DecisionModel() {
  return (
    <section className="site-section">
      <div className="site-container py-16 lg:py-20">
        <div className="grid gap-8 lg:grid-cols-[0.7fr_1.3fr]">
          <div>
            <p className="section-label mb-5">{'// DECISION_MODEL'}</p>
            <h2 className="font-mono text-3xl uppercase leading-tight text-off-white">
              THE SYSTEM THAT CREATES WORK
              <br />
              <span className="text-anvil">SHOULD NOT JUDGE IT ALONE.</span>
            </h2>
          </div>
          <div className="font-sans text-base leading-7 text-ghost-grey">
            <p>
              Every decision is about one specific action. AI can help explain the change, interpret
              a finding, or fix the problem. Deterministic software makes the final call at the
              trust boundary, and it is never the model that wrote the code.
            </p>
            <p className="mt-4 text-sm text-ghost-grey/80">
              This is the model anvil is built toward. Intent and receipts are the stages still
              being completed; the delivery boundary below draws that line.
            </p>
          </div>
        </div>

        <div className="mt-12 grid gap-3 lg:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1.15fr_auto_1fr] lg:items-center">
          {MODEL_STEPS.map(([title, description], index) => (
            <div key={title} className="contents">
              <div className="h-full border border-structure bg-surface p-4 text-center">
                <h3 className="font-mono text-xs text-off-white">{title}</h3>
                <p className="mt-2 font-sans text-xs leading-5 text-ghost-grey">{description}</p>
              </div>
              {index < MODEL_STEPS.length - 1 ? (
                <div
                  aria-hidden="true"
                  className="hidden font-mono text-lg text-ghost-grey lg:block"
                >
                  {index < 2 ? '+' : '→'}
                </div>
              ) : null}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
