export function CompanyBand() {
  return (
    <section className="site-section">
      <div className="site-container grid gap-10 py-16 md:grid-cols-[0.55fr_1.05fr_0.9fr] md:items-center lg:py-20">
        <div className="font-mono">
          <div className="text-edda">eddacraft</div>
          <div className="mt-4 text-anvil">[ = ]</div>
        </div>
        <h2 className="border-l border-border-strong pl-8 font-mono text-3xl uppercase leading-tight text-off-white sm:text-4xl">
          TRUST INFRASTRUCTURE
          <br />
          <span className="text-edda">FOR AI-ASSISTED WORK.</span>
        </h2>
        <div className="font-sans text-base leading-7 text-ghost-grey">
          <p>
            eddacraft builds technology that makes AI-assisted work independently trustworthy. anvil
            begins with software engineering.
          </p>
          <a
            href="https://eddacraft.ai"
            className="mt-5 inline-block font-mono text-sm text-edda transition-colors hover:text-off-white"
          >
            eddacraft.ai
          </a>
        </div>
      </div>
    </section>
  );
}
