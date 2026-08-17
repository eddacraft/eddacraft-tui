const MOBILE_STAGES = [
  ['UNDERSTAND', 'resident graph and applicable context', true],
  ['BUILD', 'proposed change and minimum evidence', true],
  ['DECIDE', 'deterministic constraints and enforcement', true],
  ['LEARN', 'outcomes improve future understanding', false],
] as const;

function FlywheelNode({
  className,
  title,
  description,
  current,
}: {
  className: string;
  title: string;
  description: string;
  current: boolean;
}) {
  return (
    <div
      className={`absolute z-10 w-40 border bg-void p-4 text-center font-mono ${
        current ? 'border-anvil' : 'border-border-strong'
      } ${className}`}
    >
      <h3 className={current ? 'text-anvil' : 'text-off-white'}>{title}</h3>
      <p className="mt-2 text-[10px] leading-relaxed text-ghost-grey">{description}</p>
    </div>
  );
}

export function DecisionIntegrityFlywheel() {
  return (
    <section id="system" className="site-section">
      <div className="site-container grid gap-12 py-16 lg:grid-cols-[0.72fr_1.28fr] lg:items-center lg:py-24">
        <div>
          <p className="section-label mb-5">{'// TARGET_PRODUCT_SYSTEM'}</p>
          <h2 className="font-mono text-3xl uppercase leading-tight text-off-white sm:text-4xl">
            DECISION INTEGRITY
            <br />
            <span className="text-anvil">FLYWHEEL</span>
          </h2>
          <p className="mt-6 max-w-md font-sans text-base leading-7 text-ghost-grey">
            Every decision makes the next one better. The graph and protection path provide the
            operating foundation; the wider evidence and learning loop is still being built.
          </p>

          <div className="mt-8 space-y-3 font-mono text-xs text-ghost-grey">
            <div className="flex items-center gap-3">
              <span className="h-4 w-4 border border-anvil bg-anvil" />
              <span>operating foundation</span>
            </div>
            <div className="flex items-center gap-3">
              <span className="h-4 w-4 border border-border-strong" />
              <span>system being completed</span>
            </div>
          </div>
        </div>

        <div
          className="hidden min-h-[31rem] md:relative md:block"
          role="img"
          aria-label="Decision Integrity flywheel: Understand, Build, Decide and Learn. Understand, Build and Decide have operating foundations. The wider Learn loop is being completed."
        >
          <svg aria-hidden="true" viewBox="0 0 700 500" className="absolute inset-0 h-full w-full">
            <defs>
              <marker
                id="arrow-current"
                markerWidth="8"
                markerHeight="8"
                refX="7"
                refY="4"
                orient="auto"
              >
                <path d="M0 0L8 4L0 8Z" fill="#CC5500" />
              </marker>
              <marker
                id="arrow-future"
                markerWidth="8"
                markerHeight="8"
                refX="7"
                refY="4"
                orient="auto"
              >
                <path d="M0 0L8 4L0 8Z" fill="#85858A" />
              </marker>
            </defs>
            <path
              d="M350 78 C520 78 600 150 610 244"
              fill="none"
              stroke="#CC5500"
              strokeWidth="2"
              markerEnd="url(#arrow-current)"
            />
            <path
              d="M610 270 C600 390 520 430 366 430"
              fill="none"
              stroke="#CC5500"
              strokeWidth="2"
              markerEnd="url(#arrow-current)"
            />
            <path
              d="M334 430 C170 430 100 380 90 278"
              fill="none"
              stroke="#85858A"
              strokeWidth="2"
              markerEnd="url(#arrow-future)"
            />
            <path
              d="M90 240 C100 140 178 78 334 78"
              fill="none"
              stroke="#85858A"
              strokeWidth="2"
              markerEnd="url(#arrow-future)"
            />
          </svg>

          <FlywheelNode
            className="left-1/2 top-4 -translate-x-1/2"
            title="UNDERSTAND"
            description="resident graph and applicable context"
            current
          />
          <FlywheelNode
            className="right-0 top-1/2 -translate-y-1/2"
            title="BUILD"
            description="proposed change and minimum evidence"
            current
          />
          <FlywheelNode
            className="bottom-2 left-1/2 -translate-x-1/2"
            title="DECIDE"
            description="deterministic constraints and enforcement"
            current
          />
          <FlywheelNode
            className="left-0 top-1/2 -translate-y-1/2"
            title="LEARN"
            description="outcomes improve future understanding"
            current={false}
          />

          <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 text-center font-mono text-lg uppercase leading-tight text-off-white">
            DECISION
            <br />
            INTEGRITY
          </div>
        </div>

        <ol className="space-y-3 md:hidden">
          {MOBILE_STAGES.map(([title, description, current], index) => (
            <li
              key={title}
              className={`border p-4 font-mono ${current ? 'border-anvil' : 'border-border-strong'}`}
            >
              <div className="flex items-center justify-between">
                <span className={current ? 'text-anvil' : 'text-off-white'}>
                  [{index + 1}] {title}
                </span>
                <span className="text-ghost-grey">
                  {index < MOBILE_STAGES.length - 1 ? '↓' : '↺'}
                </span>
              </div>
              <p className="mt-2 text-xs leading-relaxed text-ghost-grey">{description}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
