'use client';

const integrations = ['CURSOR', 'GITHUB COPILOT', 'CLAUDE CODE', 'WARP'];

const features = [
  {
    icon: 'O',
    name: 'WATCH_MODE',
    description:
      "Governance usually happens at CI (too late). anvil's watcher runs locally, validating context the moment code is generated.",
    highlighted: false,
  },
  {
    icon: '*',
    name: 'AGENT_AGNOSTIC',
    description:
      "Your team is fragmented. Some use Cursor, others Copilot. anvil doesn't care. We govern the output, standardising quality regardless of the agent that wrote it.",
    highlighted: false,
  },
  {
    icon: '=',
    name: 'DETERMINISTIC_ENGINE',
    description:
      'AI is inherently non-deterministic. Infrastructure must be deterministic. We force the former to behave like the latter. anvil provides the mathematical boundary that turns probabilistic code into verification.',
    highlighted: true,
  },
  {
    icon: '!',
    name: 'POLICY_AS_CODE',
    description:
      'Define standards in Rego. Version control your governance. If code violates the policy (circular deps, secrets, patterns), it never leaves localhost.',
    highlighted: false,
  },
  {
    icon: '#',
    name: 'VISUAL_BLAST_RADIUS',
    description:
      "Don't just read diffs; see the architecture. Generates interactive HTML dependency graphs to visualize risk before you commit.",
    highlighted: false,
  },
  {
    icon: '@',
    name: 'IMMUTABLE_PROVENANCE',
    description:
      'Full audit trails for every generation. Prove who wrote it, what prompt was used, and which policy validated it.',
    highlighted: false,
  },
];

export function FeatureGrid() {
  return (
    <section className="border-t border-structure bg-void">
      <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12 sm:py-16 lg:py-24">
        {/* Integration Bar */}
        <div className="mb-8 sm:mb-12 pb-3 sm:pb-4 border-b border-structure flex flex-col sm:flex-row flex-wrap gap-3 sm:gap-6 lg:gap-8 font-mono text-xs sm:text-sm uppercase text-text-muted">
          <span className="text-text-primary">INTEGRATION AGNOSTIC:</span>
          <div className="flex flex-wrap gap-3 sm:gap-6 lg:gap-8 items-center">
            {integrations.map((integration) => (
              <span key={integration} className="text-[10px] sm:text-xs">
                {integration}
              </span>
            ))}
          </div>
          <span className="text-text-muted/50 sm:border-l sm:border-structure sm:pl-6 text-[10px] sm:text-xs">
            Policy Engine: OPA / Rego
          </span>
        </div>

        {/* Header */}
        <div className="mb-8 sm:mb-12 lg:mb-16">
          <h2 className="font-mono text-xs sm:text-sm uppercase tracking-wider text-text-muted">
            {'// SYSTEM_CAPABILITIES'}
          </h2>
        </div>

        {/* Grid */}
        <div className="grid gap-4 sm:gap-px sm:bg-structure md:grid-cols-2 lg:grid-cols-3">
          {features.map((feature) => (
            <div
              key={feature.name}
              className={`group bg-void p-4 sm:p-6 transition-colors hover:bg-surface border border-structure sm:-m-px ${
                feature.highlighted ? 'bg-surface/50' : ''
              }`}
            >
              <div className="flex items-start gap-3 sm:gap-4">
                <span
                  className={`font-mono shrink-0 text-xs sm:text-sm ${feature.highlighted ? 'text-anvil' : 'text-text-muted'}`}
                >
                  [ {feature.icon} ]
                </span>
                <div className="space-y-2 sm:space-y-3 min-w-0">
                  <h3 className="font-mono text-xs sm:text-sm text-text-primary">{feature.name}</h3>
                  <p className="font-sans text-xs sm:text-sm leading-relaxed text-text-muted">
                    {feature.description}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
