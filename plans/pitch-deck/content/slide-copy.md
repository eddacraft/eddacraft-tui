# Slide Copy: Anvil Pitch Deck

---

## Slide 1: Title

**Headline**: AI governance for developers
**Subhead**: Anvil by EddaCraft

**Body**:
- Deterministic policy enforcement at file save
- Line-level authorship attribution
- Architecture drift detection
- Policy-as-code via OPA/Rego

**Data callout**: N/A -- brand moment

**Presenter notes**: "Anvil is a governance engine for AI-assisted codebases. It enforces policy at the moment code is generated -- at file save -- not after commit. Think of it as the constitutional layer for your repository. We will show you why this matters now, how it works, and why no other tool does what Anvil does."

---

## Slide 2: The problem

**Headline**: AI writes half the code. Nobody governs it.

**Body**:
- 46% of production code is now AI-generated
- AI code produces 1.7x more defects than human code
- Fewer than half of developers review AI output before committing
- 45% of AI-generated code fails security tests
- Code duplication has increased 4x since AI adoption

**Data callout**: 1.7x -- the defect multiplier for AI-generated code

**Presenter notes**: "AI coding tools are mainstream -- 84% of developers use them, 90% of the Fortune 100 have adopted Copilot. But the data is clear: AI-generated code is measurably lower quality. CodeRabbit analysed thousands of pull requests and found 1.7 times more issues in AI-generated PRs. GitClear found code duplication has quadrupled. The productivity gains are real, but so is the quality crisis. And here is the structural problem: every governance tool in the market scans after commit. By then, the ungoverned code is already in the codebase."

---

## Slide 3: Why now

**Headline**: The compliance clock is ticking

**Body**:
- EU AI Act high-risk requirements enforceable August 2026 -- 5 months away
- Penalties: up to 7% of global annual turnover
- Gartner: 40% of AI coding projects cancelled by 2027
- AI governance platform spend: USD 492M (2026), >USD 1B by 2030
- 75% of tech leaders face moderate to severe AI technical debt

**Data callout**: August 2026 -- the EU AI Act enforcement deadline

**Presenter notes**: "Three forces are converging. First, regulatory deadlines -- the EU AI Act high-risk requirements become enforceable in August 2026. That is five months from now. Non-compliance penalties reach 7% of global turnover. Second, market reality -- Gartner predicts 40% of AI coding projects will be cancelled by 2027 due to escalating costs and weak governance. Third, budget creation -- Gartner forecasts AI governance platform spend at nearly half a billion dollars this year, growing past a billion by 2030. The market is moving from 'should we govern AI code?' to 'how do we govern AI code?' We have the answer."

---

## Slide 4: The solution

**Headline**: Deterministic governance at file save

**Body**:
- Policy enforcement at the moment code is generated, not after commit
- Deterministic analysis -- not AI reviewing AI
- Line-level authorship: human, AI, mixed, or unknown
- Architecture drift detection via persistent semantic graph
- Policy-as-code (OPA/Rego) -- your team controls the rules

**Data callout**: Pre-commit -- the only governance tool at this position in the workflow

**Presenter notes**: "Anvil enforces governance at file save -- the moment code is generated. This is architecturally different from every other tool in the market. Static analysers scan after commit. AI review tools evaluate at PR time. Anvil operates at generation time. And critically, Anvil is deterministic. It uses policy-as-code -- OPA and Rego -- not another AI model. The same input always produces the same output. No probabilistic uncertainty. No AI reviewing AI. Every line of code is classified: human, AI, mixed, or unknown. And the architecture of the codebase is tracked incrementally, so drift is detected as a trajectory, not just a violation."

---

## Slide 5: How it works

**Headline**: File save to governance event in milliseconds

**Body**:
- File save triggers incremental parse (tree-sitter, Rust)
- Authorship attribution classifies each line (human / AI / mixed / unknown)
- Policy evaluation runs against team-defined OPA/Rego rules
- Architecture graph updates (dependency, trust, plan graphs)
- Governance events emitted: pass, warn, or block

**Data callout**: `save -> parse -> attribute -> evaluate -> govern`

**Presenter notes**: "Here is the technical flow. When a developer saves a file, Anvil parses it incrementally using tree-sitter in Rust -- fast enough to be synchronous. Each line is attributed: was this written by a human, an AI assistant, or some combination? Then the policy engine evaluates the change against your team's rules -- these are standard OPA/Rego policies, not proprietary. The architecture graph updates: has this change introduced a new dependency? Crossed a boundary? Expanded the trust surface? Finally, Anvil emits a governance event -- pass, warn, or block. The entire loop runs in milliseconds. No workflow disruption."

---

## Slide 6: Product

**Headline**: Built in Rust. Runs in your terminal.

**Body**:
- Terminal UI (Ratatui) -- real-time governance watcher
- CLI for CI/CD integration
- OPA/Rego policy authoring
- Architecture graph visualisation
- [Visual: TUI screenshot showing live policy enforcement]

**Data callout**: N/A -- visual proof

**Presenter notes**: "This is Anvil running in the terminal. The left pane shows the active policy -- the rules your team has defined. The right pane is the real-time signal interceptor -- it shows governance events as they happen. File saved, policy evaluated, architecture checked. The footer shows system logs. The product is built in Rust for performance and ships as a single binary. No runtime dependencies, no Docker containers, no cloud accounts required. It runs where your code runs."

---

## Slide 7: Market opportunity

**Headline**: USD 21.5B market. USD 492M in AI governance alone.

**Body**:
- TAM: USD 21.5B (AI code tools + application security + governance, 2025)
- AI governance platforms: USD 492M (2026), growing to >USD 1B by 2030 (Gartner)
- Application security testing: USD 13.6B (2025), 22%+ CAGR
- Regulatory pressure creates mandatory spend, not discretionary
- 3.4x more effective governance with purpose-built platforms (Gartner)

**Data callout**: USD 492M -- Gartner's 2026 AI governance platform forecast

**Presenter notes**: "The market sits at the intersection of three segments: AI code tools at 7.4 billion, application security testing at 13.6 billion, and AI governance platforms at 492 million and growing rapidly. The critical insight is that AI governance spend is not discretionary -- it is driven by regulatory deadlines and enterprise compliance requirements. Gartner's survey of 360 organisations found that those with governance platforms are 3.4 times more effective. This is not a nice-to-have; it is becoming infrastructure."

---

## Slide 8: Competitive landscape

**Headline**: The only tool that is both deterministic and pre-commit

**Body**:
- Static analysis tools: post-commit, no authorship attribution
- AI code review tools: probabilistic, no enforcement
- Supply chain security: covers dependencies, not code provenance
- AI governance platforms: organisational, not developer-facing
- Anvil: deterministic + pre-commit + attribution -- unique position

**Data callout**: [Visual: 2x2 positioning matrix]

**Presenter notes**: "This matrix shows the landscape on two axes: when governance happens -- pre-commit or post-commit -- and how it works -- deterministic policy or probabilistic AI. Static analysis tools like SonarQube and Semgrep are deterministic but post-commit. AI code review tools are probabilistic and post-commit. The top-left quadrant -- deterministic and pre-commit -- is empty. That is where Anvil sits. This is not a marginal improvement on an existing category. It is a structurally different approach. And moving into this quadrant requires fundamental re-architecture, not a feature addition."

---

## Slide 9: Business model

**Headline**: Land with developers. Expand with compliance.

**Body**:
- Developer-led adoption: CLI install, bottom-up within teams
- Per-seat pricing: team tier + enterprise tier
- Expansion via policy packs: compliance-specific rule sets (SOC 2, HIPAA, EU AI Act)
- Enterprise upsell: centralised policy management, audit dashboards, SSO
- Open-source core for community adoption; commercial features for governance

**Data callout**: Land and expand -- developer adoption to enterprise compliance

**Presenter notes**: "The go-to-market follows the developer tools playbook. A developer installs Anvil via CLI, configures a few policies, and sees immediate value -- governance events on every save. That is the land. The expand happens when compliance requirements arrive: the EU AI Act deadline, a SOC 2 audit, an enterprise customer asking about AI code governance. Policy packs -- pre-built rule sets for specific compliance frameworks -- become the expansion revenue. Enterprise features like centralised policy management and audit dashboards create the upsell."

---

## Slide 10: Traction

**Headline**: [EVIDENCE NEEDED]
**Subhead**: [EVIDENCE NEEDED -- waitlist numbers, design partners, community metrics]

**Body**:
- [EVIDENCE NEEDED: waitlist size and growth rate]
- [EVIDENCE NEEDED: design partner count and profile]
- [EVIDENCE NEEDED: open source community metrics (stars, contributors, downloads)]
- Product maturity: Rust kernel, Ratatui TUI, OPA/Rego engine, authorship attribution
- Technical validation: persistent semantic graph, incremental analysis operational

**Data callout**: [EVIDENCE NEEDED]

**Presenter notes**: "[To be written when traction data is available. Key talking points should include: early market validation signals, product maturity (this is not a prototype), and any enterprise interest or design partner feedback.]"

---

## Slide 11: Team

**Headline**: [EVIDENCE NEEDED]

**Body**:
- [EVIDENCE NEEDED: founder bios, relevant experience]
- [EVIDENCE NEEDED: team composition and key hires]
- [EVIDENCE NEEDED: advisors]

**Data callout**: [EVIDENCE NEEDED]

**Presenter notes**: "[To be written when team information is provided. Key talking points should include: domain expertise in developer tooling and governance, technical depth in systems programming (Rust), and understanding of the enterprise compliance landscape.]"

---

## Slide 12: The ask

**Headline**: [EVIDENCE NEEDED -- funding amount]

**Body**:
- [EVIDENCE NEEDED: funding amount]
- [EVIDENCE NEEDED: use of funds breakdown]
- [EVIDENCE NEEDED: key milestones this funding enables]
- [EVIDENCE NEEDED: timeline to next raise / revenue milestones]

**Data callout**: [EVIDENCE NEEDED]

**Presenter notes**: "[To be written when funding details are provided. Structure: amount, use of funds (engineering, go-to-market, compliance certification), milestones (GA launch, first enterprise customers, ARR targets), and what this positions the company for at the next stage.]"

---

## Slide 13: Appendix

**Headline**: Deep dive materials

**Body**:
- A: Technical architecture -- Rust kernel, semantic graph, OPA integration, Ratatui TUI
- B: Detailed competitive comparison -- feature-by-feature across categories
- C: Regulatory timeline -- EU AI Act, US frameworks, enterprise compliance deadlines
- D: Financial model assumptions -- pricing, adoption curve, revenue projections
- E: Product roadmap -- near-term (6 months) and medium-term (18 months)

**Data callout**: N/A

**Presenter notes**: "These slides are here for Q&A and follow-up. Each one provides the detailed evidence behind a claim in the main deck. The technical architecture slide shows how the Rust kernel, semantic graph, and OPA engine work together. The competitive comparison goes feature-by-feature. The regulatory timeline shows every deadline through 2030. Use these to answer deep questions with confidence."
