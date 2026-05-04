# Pitch Deck Narrative Synthesis

## Core Thesis

Anvil is a deterministic governance layer for AI-assisted software development.

The company is not fundamentally a linter, CI tool, code reviewer, or AI agent
platform. Its strongest interpretation is: **the control layer that sits between
AI-generated change and production code, enforcing architectural, security, and
organisational rules at the moment code is created.**

Plain-language problem: AI coding tools let teams generate more code than humans
can reliably review. The result is not just obvious bugs, but slow structural
decay: architecture drift, suppressed guardrails, hidden security risk,
undocumented exceptions, and code that technically works while weakening the
system.

Who it is for: initially AI-heavy engineering teams with meaningful architecture
to protect, especially SaaS, fintech, healthcare, regulated software, and
platform engineering organisations. The user wedge is the individual developer
using AI tools. The economic buyer is the CTO, VP Engineering, platform lead, or
compliance-conscious engineering leader.

Why now: AI code generation has moved from experiment to production workflow.
The bottleneck has shifted from "can we generate code?" to "can we trust what
gets generated?" Existing controls fire too late: PR review, CI, security
scanning, or audit evidence after the fact. Regulation and enterprise AI-risk
policies are turning this from best practice into budgeted spend.

## Problem

AI coding assistants have collapsed the cost of generating software, but they
have not collapsed the cost of governing it.

The real failure mode is not "AI writes bad code." That is too generic. The
sharper problem is: **AI writes plausible code that passes local checks while
eroding architecture, policy, and institutional intent.**

Examples:

- A feature reaches across domain boundaries because the model found an import
  that works.
- A tool adds `any`, disables lint rules, swallows errors, or leaves vague TODOs
  to make the patch pass.
- A codebase accumulates new dependency edges that nobody consciously approved.
- Reviewers see more code than they can deeply inspect.
- Compliance and provenance questions appear after merge, when reconstruction is
  expensive.

The painful insight is that traditional governance operates after creation.
Anvil moves governance into the creation loop.

## Solution

Anvil validates software changes before they become committed reality.

At a high level, Anvil:

- Watches file changes and proposed AI writes.
- Builds and maintains structural understanding of the codebase.
- Runs deterministic checks for architecture boundaries, anti-patterns, secrets,
  command safety, and policy-as-code.
- Produces structured diagnostics that humans and AI tools can consume.
- Allows intentional exceptions only with explicit, human-owned suppressions.
- Creates evidence of what was checked, against which rules, and with what
  result.

The current product surface is a Rust CLI and validation engine, with watch
mode, gate/check commands, an AI guardrail profile, MCP integration, policy
support via OPA/Rego, architecture boundary checks, secret detection,
anti-pattern rules, and early real-time validation for AI tool writes.

The strategic product is bigger: **a deterministic control plane for software
change**, where AI agents, IDEs, terminals, CI systems, dashboards, and
enterprise policy workflows all consume the same validation substrate.

What makes it different:

- It is deterministic, not AI reviewing AI.
- It acts pre-commit and increasingly pre-write, not after PR creation.
- It focuses on architecture and intent drift, not just syntax, style, or known
  vulnerabilities.
- It turns governance into a developer workflow primitive rather than an audit
  artefact.
- It is designed for low-latency local execution, not only centralised SaaS
  scanning.

## Unique Insight

The company's non-obvious belief is:

**The winning governance layer for AI software development will not be another
AI reviewer. It will be a deterministic, local-first control system that
prevents bad changes at the point of creation.**

Most competitors and incumbents treat AI risk as a review, security, compliance,
or observability problem. Anvil treats it as a control-loop problem.

The team understands that AI adoption creates a timing mismatch. If you wait
until PR review, CI, or audit, the developer has already accepted the generated
shape of the solution. The cheap moment to intervene is while the change is
being created, when the developer or agent can still re-plan.

The second insight is that architecture drift is the leading indicator. Bugs are
discrete; drift is cumulative. The most damaging AI-generated failures are often
small local compromises that compound into future velocity loss, security
exposure, and governance ambiguity.

## Market

The strongest category framing is: **AI governance for software engineering**.

This is adjacent to, but distinct from:

- Static analysis
- AppSec
- Code review automation
- DevSecOps
- AI governance platforms
- Agent security
- Software supply chain provenance

The initial segment should be tightly framed:

- AI-heavy SaaS engineering teams with 5-200 developers.
- Platform teams responsible for architecture consistency across multiple repos.
- Regulated software teams where provenance and policy evidence matter.
- Engineering leaders adopting Cursor, Claude Code, Copilot, or agentic
  workflows but worried about loss of control.

The wedge is developer-led adoption: install the CLI, wire it into the
editor/MCP/CI loop, get immediate warnings. The expansion motion is enterprise
governance: team dashboards, policy packs, multi-repo policy federation,
compliance evidence, trust centre automation, and central management.

Why this can be a big market:

- AI coding adoption is becoming default developer infrastructure.
- Governance spend follows risk concentration.
- The buyer already pays for AppSec, CI, code quality, compliance, and developer
  productivity.
- Anvil sits at the intersection of those budgets but has a sharper trigger: "we
  are letting AI write production code and need provable control."

Avoid over-relying on broad TAM. The better investor argument is that AI code
generation creates a new mandatory control layer, just as cloud created cloud
security posture management and DevOps created CI/CD governance.

## Differentiation & Defensibility

The strongest differentiation is architectural, not messaging.

Anvil's defensible choices:

- **Deterministic by design:** no AI inside the control decision. This is
  credible in regulated or high-trust environments.
- **Pre-commit/pre-write placement:** acting at save-time or proposed-write time
  requires different architecture from PR scanners.
- **Rust local engine:** supports low-latency validation, local-first workflows,
  and single-binary distribution.
- **Policy-as-code foundation:** OPA/Rego gives enterprise extensibility and
  compliance alignment.
- **Structural graph direction:** persistent
  semantic/dependency/trust/provenance graphs can become the strategic moat if
  executed.
- **Evidence and provenance model:** replayable/signed attestations can turn
  developer workflow into audit-grade proof.
- **Workflow lock-in through suppressions and policy:** once teams encode
  architecture, exceptions, evidence, and policy history into Anvil, switching
  costs increase.

Be sceptical: the moat is not yet fully proven. Static analysis, security
vendors, IDE vendors, and AI coding platforms could copy parts of the surface.
The defensibility depends on Anvil getting deep enough into the local control
loop, graph substrate, evidence model, and developer workflow before incumbents
bolt on shallow "AI governance" features.

The strongest claim is not "nobody can copy checks." They can. The stronger
claim is: **post-commit tools cannot become pre-write deterministic governance
without rebuilding their architecture and developer workflow assumptions.**

## Product/Architecture Notes

Strategically important architecture:

- **Rust CLI/kernel:** gives speed, local execution, low-latency watch mode, and
  credible "not vibe-coded" positioning.
- **Checks/findings/gates model:** separates individual validations from
  workflow judgement, making the product composable across CLI, CI, MCP, editor,
  and dashboard.
- **AI guardrail profile:** a curated strict mode for AI workflows, with stable
  JSON diagnostics that tools and agents can consume.
- **MCP pre-write validation:** important because it moves Anvil from "developer
  sees warning" to "agent receives structured refusal or warning before
  writing."
- **Intercept daemon and surface drivers:** the right long-term control-plane
  architecture, though still in progress.
- **Graph v2 substrate:** if delivered, this becomes the real strategic
  foundation: joined semantic, dependency, trust, control/session, and
  provenance graphs.
- **Verifiable governance attestations:** strategically converts engineering
  hygiene into compliance evidence.

The architecture matters because Anvil is trying to own the moment of change,
not just analyse the repository afterwards.

## Risks & Open Questions

Key risks:

- **Positioning risk:** internal docs oscillate between developer trust,
  enterprise governance, compliance, and broad "knowledge worker governance."
  The deck should stay focused on AI software governance. Phase 2
  knowledge-worker governance should be a small future note, not the main story.
- **Product maturity risk:** some claims are stronger than the shipped state.
  Real-time validation exists through the Rust MCP launch path, but the
  daemon-backed path and full editor driver story are still in progress.
- **Authorship/provenance risk:** line-level authorship and full evidence
  commands appear partly planned or incomplete. Do not overclaim "every line
  classified" unless current implementation supports it end-to-end.
- **Enforcement contradiction:** docs say "warnings over blocks" while vision
  says "prevent/block unsafe changes." Resolve this as: local developer mode is
  advisory by default; AI/pre-write and enterprise policy modes can enforce.
- **Market evidence risk:** TAM and defect/security statistics need external
  citation hygiene before investor use.
- **Adoption risk:** developers may resist another tool unless first-run value
  is immediate and false positives are low.
- **Incumbent risk:** GitHub, Snyk, Semgrep, Sourcegraph, Cursor, or security
  platforms could add AI governance narratives quickly.
- **Complexity risk:** the roadmap is broad. The company must avoid looking like
  a sprawling internal platform project.
- **Buyer clarity risk:** individual developers benefit first, but enterprises
  pay. The deck must clearly explain land-and-expand.

Investor concern to expect: "Is this a feature or a company?" The answer must
be: it is a company if Anvil owns the control point for AI-generated software
change and becomes the evidence layer for whether AI-assisted engineering is
safe to scale.

## Narrative (Investor-Ready)

Software development has entered a new phase. AI can now generate production
code faster than engineering organisations can review it. The generation layer
is no longer the bottleneck. Trust is.

The problem is not that AI code always fails. The deeper problem is that it
often succeeds locally while weakening the system globally. It imports across
boundaries, suppresses guardrails, hides errors, leaks secrets, and accumulates
architectural debt in small increments. Traditional tools catch some of this
later, in PR review, CI, security scanning, or audit. By then the change has
already been accepted into the workflow and the cost of correction is much
higher.

Anvil moves governance to the moment of creation. It validates changes at
save-time and, increasingly, before AI tools write to disk. It gives developers
and agents immediate, structured feedback when a proposed change violates
architecture, policy, security, or organisational constraints.

The critical distinction is that Anvil is deterministic. It does not ask
companies to trust another AI model to review the output of their coding AI. It
runs mechanical, reproducible checks through a Rust engine, policy-as-code,
architecture graphs, and structured diagnostics. The same input produces the
same result. That matters when the output is used to make engineering, security,
and compliance decisions.

The wedge is simple: developers want to use AI coding tools without becoming the
cleanup crew for invisible architecture drift. Anvil gives them a local, fast,
actionable guardrail. The expansion is enterprise-grade: platform teams define
policy, engineering leaders track drift, compliance teams get evidence, and
organisations gain a governance layer for AI-assisted development.

The timing is unusually sharp. AI coding assistants are already embedded in
production engineering workflows, but governance has not caught up. Enterprises
are beginning to ask who wrote what, which controls ran, whether policy was
enforced, and whether AI-generated changes can be trusted. Regulation and
internal AI-risk policies will accelerate that demand.

Anvil's long-term advantage is not a list of checks. It is the control loop:
local-first validation, pre-write integration, deterministic policy, structural
graph intelligence, human-owned exceptions, and verifiable evidence. That
combination turns governance from an after-the-fact audit process into an
operating system for safe software change.

The company should therefore be pitched as the deterministic governance layer
for AI-generated software. Not a linter. Not another AI reviewer. Not a CI
plugin. The adult control plane between AI code generation and production trust.

## Suggested Slide Titles

1. AI Writes the Code. Who Governs It?
2. The New Bottleneck Is Trust
3. Existing Controls Fire Too Late
4. Anvil: Deterministic Governance at Creation Time
5. How It Works: From Proposed Change to Governance Event
6. No AI Inside: Why Deterministic Wins
7. The Beachhead: AI-Heavy Engineering Teams
8. Why We Win: Pre-Write, Local, Policy-Driven
9. Land with Developers, Expand with Governance
10. Built Now to Own the Control Layer
