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

**Headline**: Built in Rust. No AI inside. 50ms per check.

**Body**:
- Terminal UI (Ratatui) -- real-time governance watcher
- CLI for CI/CD integration
- OPA/Rego policy authoring
- Architecture graph visualisation
- [Visual: TUI screenshot showing live policy enforcement]

**Data callout**: N/A -- visual proof

**Presenter notes**: "This is Anvil running in the terminal. The left pane shows the active policy -- the rules your team has defined. The right pane is the real-time signal interceptor -- it shows governance events as they happen. File saved, policy evaluated, architecture checked. The footer shows system logs. The product is built in Rust for performance and ships as a single binary. No runtime dependencies, no Docker containers, no cloud accounts required. It runs where your code runs. Every check is deterministic -- programmatic, mechanical, repeatable. No AI reviewing AI. The same input always produces the same output. This product plays in the exact space AI struggles with: precision. And it runs in under 50 milliseconds."

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

**Headline**: Built what others are pitching

**Body**:
- Production Rust kernel with 6 crates, 50ms deterministic checks
- OPA/Rego policy engine with line-level authorship attribution
- Persistent semantic graph for architecture drift detection
- 5 pilot teams engaged, targeting 10-15 by close
- Waitlist targeting 5,000+ with developer influencer demos in pipeline
- Enterprise pipeline via Arkahna's 100+ SaaS client network
- 2 open source packages released

**Data callout**: Competitors in this category are raising on decks. Anvil is raising on a working product. Not vibe-coded -- precision-engineered in a domain where AI fails.

**Presenter notes**: "While other companies in the AI governance space are raising record rounds on pitch decks and prototypes, Anvil is a production-grade system. The Rust kernel, the policy engine, the semantic graph, the authorship attribution -- all built. Precision-engineered in a domain where AI struggles: deterministic analysis, sub-50-millisecond checks, repeatable results. We have 5 pilot teams today and developer influencers lined up to demo ahead of launch. We're targeting 5,000 on the waitlist and 10 to 15 pilot teams by the time we close this round. The product plays in the exact space AI fails at -- precision -- and that's the point."

---

## Slide 11: Team

**Headline**: 25 years building what enterprises buy

**Body**:
- Joshua Boys, Founder and CEO
- Former Microsoft Azure Lead, Australia
- CEO of Arkahna -- platform engineering for 100+ SaaS companies over 5 years
- 25+ years building enterprise software, leading teams, shipping SaaS
- Advisory bench: senior advisors across enterprise software, startup scaling, and large SaaS
- Capital efficient: £0 raised, production-grade product delivered
- First hires are engineering + enterprise-focused CRO -- the team scales with the raise

**Data callout**: Built governance tooling from inside the enterprise buying process -- not from a research lab

**Presenter notes**: "I've spent 25 years in enterprise software -- the last five as CEO of Arkahna, a platform engineering company that works with over 100 SaaS companies. I was the Azure lead in Australia for Microsoft. I know how enterprises buy developer tools, because I've been on both sides of that transaction. Anvil exists because I've watched AI coding tools arrive in my clients' organisations with zero governance. The advisory bench includes senior operators from enterprise, startups, and large SaaS. The team scales with this raise -- first hires are engineers and a CRO. Built to make you trust your AI more -- not by asking you to trust ours."

---

## Slide 12: The ask

**Headline**: Own the category before the window closes

**Body**:
- £3--5M seed round at £15--25M pre-money valuation
- Category: AI governance -- hottest new category in developer tooling
- Engineering (~40%): 3--4 hires, scale Rust kernel, platform layer, ecosystem integrations
- Go-to-Market (~30%): Enterprise-focused CRO + developer advocacy/community
- Strategic Acquisition (~20%): Acquire platform engineering IP -- clean arm's-length transactions, accelerating maturity
- Operations (~10%): Compliance certification, infrastructure
- Milestones: profitability on phase 1, 5,000+ waitlist, 50+ paying teams, enterprise contracts
- Phase 2 ready: expand to knowledge worker governance -- without further dilution

**Data callout**: EU AI Act enforcement: August 2026. 5 months to capture the compliance purchasing wave.

**Presenter notes**: "We're raising £3--5M to own this category before the compliance window closes. 40% goes to engineering -- scaling the Rust kernel, building the platform layer, and ecosystem integrations. 30% to go-to-market -- an enterprise CRO and developer advocacy to drive bottom-up adoption. 20% to strategic acquisition of platform engineering IP -- proven infrastructure we can absorb rather than rebuild, accelerating our maturity by months. This round gets us to profitability on code governance. That's phase 1. Phase 2 is the bigger thesis: AI governance for all knowledge work -- legal, finance, operations -- starting from the beachhead where the pain is sharpest and the tooling is most mature. We reach phase 2 self-funded. No further dilution required."

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
