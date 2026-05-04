# Anvil — Pitch Synthesis

## Core Thesis

Anvil is the **deterministic governance layer for AI-assisted software**. It
enforces policy at file save — the moment code is generated — using
policy-as-code, line-level authorship attribution, and a persistent semantic
graph that tracks architectural drift in real time.

Stripped down: every dollar of AI-coding investment so far has funded the
_accelerator_. Anvil is the _brake_ — and the brake cannot itself be
probabilistic, because a probabilistic brake is not a brake. That is the company
in one sentence.

## Problem

AI now writes ~46% of production code. Quality is measurably worse: 1.7×
defects, 45% fail security tests, 4× code cloning, refactoring collapsed from
25% to <10% of changed lines. Fewer than half of developers review AI output
before committing.

Every governance tool on the market — SAST, AI code review, supply-chain
scanners, GRC platforms — runs _after_ commit. By PR time, the ungoverned code
is already in the codebase, drift is already accumulating, and the cost-to-fix
has gone up 10–100×. The window between "AI generates" and "anyone notices" is
the unattended part of the modern engineering pipeline.

## Solution

Anvil intercepts every change at file save and runs it through a deterministic
loop in Rust:

```
save → tree-sitter parse → line-level authorship → OPA/Rego policy → semantic graph delta → governance event (pass / warn / block)
```

Five things make it concrete:

1. **Pre-commit, not post-commit.** Governance is synchronous with generation.
   The developer is still in the chair.
2. **Deterministic, not probabilistic.** OPA/Rego policy-as-code. Same input →
   same output. Audit-grade.
3. **Line-level authorship.** Every line is classified as human / AI / mixed /
   unknown. Provenance becomes a first-class property of the codebase, not a
   forensic reconstruction.
4. **Architecture drift as trajectory.** A persistent semantic graph
   (dependency, trust, plan graphs) updates incrementally, so "you are trending
   toward structural instability" replaces "you broke a rule."
5. **Single Rust binary.** No cloud account, no daemon to babysit, ~10 µs
   incremental check, ~800 ns full policy evaluation. Zero perceptible latency.

It runs in the terminal (CLI + Ratatui TUI), in the IDE, in MCP for agents, and
in CI — same kernel, same policies, every surface.

## Unique Insight

The non-obvious belief that drives the company:

**You cannot govern a probabilistic process with another probabilistic
process.** The industry's reflex is "use AI to review AI" — that compounds
uncertainty rather than constraining it. The only stable control plane for
AI-generated code is one that is itself deterministic, and that fires _before_
code propagates anywhere expensive.

A second, sharper insight: **governance is not a process category, it is an
architectural one.** Pre-commit + deterministic + provenance-aware cannot be
bolted onto an existing post-commit scanner; the data structures, latency
budget, and integration surface are all wrong. Whoever builds it first owns the
quadrant for years.

A third: **regulation will turn this from a 'should' into a 'must' on a fixed
date.** August 2026 (EU AI Act, high-risk Annex III) is not a trend — it is a
calendar event with 7%-of-turnover penalties attached. That converts a
developer-tools sale into a compliance sale, with the timing of one and the
price tag of the other.

## Market

- **TAM** (intersection of AI code tools, AppSec, AI governance platforms): ~USD
  21.5B (2025), trending to >USD 40B by 2030.
- **SAM** (governance/quality for AI-assisted dev): ~USD 1.5–2.0B (2026),
  inferred from segment overlaps. Caveat: no analyst report covers this exact
  intersection — derivation, not citation.
- **AI governance platforms specifically**: Gartner forecasts USD 492M (2026) →
  > USD 1B (2030). This is the cleanest single anchor.
- **Forcing function**: EU AI Act high-risk obligations enforceable **2 August
  2026** — roughly thirteen weeks from today's date (2026-05-04). Note: existing
  deck copy says "5 months"; this is stale by ~2 months and should be tightened
  to "this quarter" or "~13 weeks."

**Customer**: engineering organisations of 20–500 devs that already deploy AI
coding assistants and have an existing compliance posture (SOC 2 / ISO 27001 /
regulated industry). Buyer triangle: VP Eng/CTO economic, security/compliance
influencer, individual developer adopter. Land bottom-up via CLI; expand
top-down when an audit, an enterprise customer questionnaire, or the EU AI Act
forces the conversation.

**Category framing**: "Agentic engineering governance" — being defined right
now, parallel to Qodo's $70M Series B and DAM Secure's $4M seed (April 2026).
The category is real and is being capitalised.

## Differentiation & Defensibility

Anvil is the **only tool that is simultaneously deterministic and pre-commit,
with line-level authorship**. Every other player is in one of the other three
quadrants:

| Where they live            | Examples                   | Why they can't move                                                                   |
| -------------------------- | -------------------------- | ------------------------------------------------------------------------------------- |
| Deterministic, post-commit | SonarQube, Semgrep, Snyk   | Bolting file-save + provenance onto a CI-era architecture is a rewrite, not a feature |
| Probabilistic, post-commit | CodeRabbit, Sourcery, Qodo | Going deterministic abandons their core thesis (AI reviewing AI)                      |
| Probabilistic, pre-commit  | (empty)                    | Doesn't solve the problem — same compounding uncertainty                              |

Four moats, in order of durability:

1. **Architectural** — pre-commit + incremental semantic graph requires a Rust
   kernel and a different data model. Years of work to replicate.
2. **Provenance data** — line-level authorship needs deep, real-time workflow
   integration. Retroactive attribution is unreliable; whoever owns the live
   capture pipeline owns the dataset.
3. **Policy ecosystem** — OPA/Rego is an open standard; Anvil's expansion
   product is curated **policy packs** (SOC 2, HIPAA, EU AI Act). The packs
   become the recurring asset, not the engine.
4. **Open-standards posture** — using OPA/Rego rather than a proprietary rules
   language defuses the "vendor lock-in" objection competitors will make.

Honest weak spots in the moat story:

- OSS engine + open-standard policies cuts both ways — competitors and
  incumbents can adopt the same standards.
- A GitHub-native governance feature is the largest single threat. They have the
  data, the surface, and the distribution.
- "Deterministic" is brittle to evolving AI failure modes; the policy library
  has to keep pace, which is operational, not architectural, work.

## Product / Architecture Notes

Worth surfacing for technical investors:

- **Rust kernel** (`anvil-kernel`) with measured performance: 14.5 ms cold graph
  build on 100 files; ~10 µs incremental update; 800 ns full policy evaluation;
  ~25× under target on event emission. Released `v0.5.0-beta` on 2026-05-01 with
  parallelised scanning at ~40K artifacts/sec.
- **Mid-edit intercept daemon** (`anvil-intercept`) is the path to "guardrails
  as you type," not just on save — the strategic North Star is real-time
  invariant violation streaming.
- **Edda Stack** (Kindling → Ember → Edda) gives Anvil a learning loop for
  emergent anti-patterns without using probabilistic enforcement — observation
  feeds policy proposals, humans confirm, deterministic enforcement follows.
  This is how Anvil avoids the "rules library rots" failure mode.
- **APS (Anvil Plan Spec)** closes the loop between _intended_ change and
  _actual_ change — code that drifts outside an active plan is itself a
  governance event. This is the most genuinely novel idea in the corpus.

These four together support a credible long-term framing: Anvil is not a linter,
it is the **constitutional runtime** of the repository — an enforcement kernel
whose long-term version watches _evolution_, not just files.

## Risks & Open Questions

**Strategic**

- _Compliance vs. devtool buyer compression._ Land-with-developers +
  sell-to-compliance is the right motion, but the pricing power lives in the
  compliance buyer and the velocity lives with the developer. The team needs to
  nail the handoff or both motions stall.
- _EU AI Act applicability scope._ Penalties are real, but the set of orgs whose
  code generation is in-scope as a "high-risk AI system" is narrower than the
  deck implies. The cleaner pitch is "the regulation puts AI provenance on every
  enterprise's procurement checklist," not "every customer is directly liable."
- _Phase 2 (knowledge-worker governance)._ Currently a one-line ambition. It
  needs to either be removed from the seed deck or upgraded with concrete
  evidence; today it adds vagueness without conviction.

**Product / Technical**

- Authorship attribution accuracy at line level is the single most-cited
  capability and the least-proven. Without a published benchmark, it is a claim,
  not a moat.
- Performance numbers are Criterion micro-benchmarks. The investor question is
  end-to-end p99 on a real 2,000-file repo under sustained edit load — that data
  is "pending."
- "No AI inside" is brand-clean but creates a long-term capability ceiling. The
  roadmap needs a credible answer for emergent anti-patterns that no human
  author of policy will anticipate (the Edda Stack is the answer; it should be
  sharper in the deck).

**Traction / Team**

- 5 pilot teams + 5,000-developer waitlist target is light traction. Design
  partners from regulated industries (financial services, healthcare) are the
  missing proof.
- Solo founder. Advisory bench named but not personalised. First two hires
  (engineering, enterprise CRO) are the right shape but bench depth will be a
  question.
- £0 raised with a production product is a real story — capital-efficient
  builder. Lean into it.

**Document hygiene**

- "5 months to EU AI Act" appears throughout the deck; today is 2026-05-04 and
  the deadline is 2026-08-02 — closer to 13 weeks. Tighten before sending.
- TAM stack mixes AI-code-tools, AppSec, and AI-governance into USD 21.5B;
  investors will discount at least one layer. Lead with the cleaner Gartner
  number (USD 492M → 1B+) and use TAM as the ceiling, not the headline.

## Narrative (Investor-Ready)

In four years, AI has gone from "interesting demo" to writing half of production
code at 90% of the Fortune 100. Every venture dollar in this wave has funded the
_accelerator_ — tools that help AI write code faster. None has funded the
_brake_.

The data is now in, and it is bad. AI-generated code carries 1.7× the defect
rate, 4× the duplication, and 45% of it fails security tests. Fewer than half of
developers review it before committing. The "almost-right" code that compiles
but quietly erodes architecture is the dominant failure mode of the new
development era.

And every existing tool that might catch it — static analysers, AI code
reviewers, supply-chain scanners — runs _after_ commit, by which point the cost
of fixing is 10 to 100 times higher and the developer who wrote (or accepted)
the change has moved on. The governance gap is structural, not a tooling
oversight.

Anvil closes that gap by moving governance into the same moment as generation.
When a developer saves a file, Anvil parses it in microseconds, classifies every
line by author (human, AI, mixed, unknown), evaluates it against the team's
policy-as-code rules, updates a persistent semantic graph of the codebase's
architecture, and emits a deterministic verdict. No AI reviewing AI. No
probabilistic governance over a probabilistic process. The same input always
produces the same output — which is the only kind of result an audit can accept.

This position — _deterministic AND pre-commit, with line-level provenance_ — is
empty in the market today. Static-analysis incumbents are post-commit by
architecture. AI code reviewers are probabilistic by thesis. Supply-chain tools
live one layer outside the repo. To move into Anvil's quadrant, any of them has
to rewrite their product, not add a feature. That is the moat: not a clever
algorithm, but the choice to start from the right architecture.

The timing is not a guess. The EU AI Act's high-risk obligations become
enforceable on 2 August 2026 — about thirteen weeks from now — with penalties of
up to 7% of global turnover. Gartner forecasts AI-governance platform spend at
USD 492M this year, growing past USD 1B by 2030, and reports that organisations
with governance platforms are 3.4× more effective. This is the moment governance
budgets stop being discretionary and start being a procurement-checklist item.

Anvil is already a working product. A Rust kernel, a Ratatui terminal UI, an
OPA/Rego policy engine, a persistent semantic graph, and authorship attribution
— all built and benchmarked, with `v0.5.0-beta` shipped on 2026-05-01 and
incremental checks measured at ~10 microseconds. Competitors in the category are
raising on prototypes. We are raising on a system.

The go-to-market is the developer-tools playbook with a compliance accelerator.
A developer installs the binary, gets value in five minutes, and brings the
team. The expansion is triggered by the next audit, the next enterprise
questionnaire, or the next regulatory deadline — at which point the policy packs
(SOC 2, HIPAA, EU AI Act) and the enterprise control plane become the line item.
Land with developers; expand with compliance.

The founder profile is the missing piece in most pitches in this category: 25
years inside the enterprise buying process, ex-Microsoft Azure lead, CEO of a
platform-engineering company that already serves 100+ SaaS customers. Anvil was
built from the buyer's seat, not from a lab.

We are raising £3–5M at £15–25M pre-money to take the empty quadrant before the
window closes. The round funds engineering depth, an enterprise CRO, and one
strategic acquisition of platform-engineering IP that compresses our maturity by
months. It carries us to profitability on Phase 1 — code governance — and
positions us to fund Phase 2 (governance of broader knowledge work) without
further dilution.

The category exists because the governance gap is real and measurable. The
window exists because regulation has put a date on it. The defensibility exists
because moving into this quadrant is a re-architecture, not a feature. We have
the product, the timing, and the buying-side instinct to take it.

---

## Suggested Slide Titles (max 10)

1. **AI writes half the code. Nobody governs it.**
2. **The brake nobody funded** — why every existing tool runs too late
3. **August 2026: the compliance clock** — EU AI Act, 7% of global turnover
4. **Anvil — deterministic governance at file save**
5. **The empty quadrant** — pre-commit × deterministic × provenance
6. **How it works** — save → parse → attribute → enforce, in microseconds
7. **Built, not pitched** — Rust kernel, ~10 µs incremental, shipped v0.5.0-beta
8. **USD 492M → USD 1B+** — Gartner's AI-governance curve
9. **Land with developers. Expand with compliance.**
10. **£3–5M to own the category before the window closes**
