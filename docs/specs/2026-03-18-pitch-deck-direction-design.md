# Pitch Deck Direction — Design Spec

**Date**: 2026-03-18 **Status**: Reviewed (spec review: approved with notes —
all addressed) **Audience**: Angel / pre-seed investors (stretch to seed)
**Approach**: Category Creator (B) with Platform Vision (C) undertone

---

## Context

The pitch deck pipeline (Pitch Orchestrator) produced 16 documents across
research, strategy, content, and synthesis phases. Slides 1–9 and 13 have
complete copy, data, and visual specs. Three slides were flagged
`[EVIDENCE NEEDED]`: Traction (10), Team (11), and The Ask (12).

This spec captures the decisions made during brainstorming to fill those gaps
and refine the overall deck direction.

---

## Core Framing

### Category Creator Energy

eddacraft/Anvil is positioned as the technical-first entrant in a white-hot new
category (AI governance for developers). The framing is: "built what others are
pitching." Competitors in the space are raising record rounds on decks and
prototypes; Anvil is raising on a production-grade product.

### "No AI Inside" Thread

A recurring beat across slides 6, 10, and 11:

> Anvil contains zero AI. Every check runs in under 50ms — deterministic,
> mechanical, programmatic. In a category full of AI reviewing AI, Anvil earns
> trust by being the thing you can verify. Precision-engineered in Rust, not
> vibe-coded. Built to make you trust your AI more — not by asking you to trust
> ours.

Placement:

- **Slide 6 (Product)**: "Built in Rust. No AI inside. 50ms per check."
- **Slide 10 (Traction)**: "Not vibe-coded. Precision-engineered in a domain
  where AI fails."
- **Slide 11 (Team)**: "Built to make you trust your AI more — not by asking you
  to trust ours."

### Regulatory Urgency

EU AI Act high-risk requirements enforceable August 2026 (5 months). This is the
purchasing trigger and creates a "window closing" narrative that runs through
slides 3, 7, 8, and 12.

### Phase 2 Vision

Code governance is phase 1 (this raise). Phase 2 is knowledge worker governance
— legal, finance, operations. Mentioned once on the Ask slide as a scale signal,
not over-explained.

---

## Slide Deck Structure (13 Slides)

| #   | Slide                 | Headline                                                |
| --- | --------------------- | ------------------------------------------------------- |
| 1   | Title                 | AI governance for developers                            |
| 2   | Problem               | AI writes half the code. Nobody governs it.             |
| 3   | Why Now               | The compliance clock is ticking                         |
| 4   | Solution              | Deterministic governance at file save                   |
| 5   | How It Works          | File save to governance event in milliseconds           |
| 6   | Product               | Built in Rust. No AI inside. 50ms per check.            |
| 7   | Market Opportunity    | USD 21.5B market. USD 492M in AI governance alone.      |
| 8   | Competitive Landscape | The only tool that is both deterministic and pre-commit |
| 9   | Business Model        | Land with developers. Expand with compliance.           |
| 10  | Traction              | Built what others are pitching                          |
| 11  | Team                  | 25 years building what enterprises buy                  |
| 12  | The Ask               | Own the category before the window closes               |
| 13  | Appendix              | Deep dive materials                                     |

---

## Gap Slides — Completed Designs

### Slide 10: Traction

**Headline**: Built what others are pitching

**Structure** (three columns):

| Built Today                  | Launch Trajectory (targets)                               | Ecosystem              |
| ---------------------------- | --------------------------------------------------------- | ---------------------- |
| Production Rust kernel       | 5,000+ waitlist target (dev influencer demos in pipeline) | 2 open source packages |
| 50ms deterministic checks    | 10–15 pilot teams target (5 currently engaged)            | Community building     |
| OPA/Rego policy engine       | Enterprise pipeline via Arkahna's 100+ SaaS network       |                        |
| Semantic graph + attribution |                                                           |                        |

**Data callout**: "Competitors in this category are raising on decks. Anvil is
raising on a working product. Not vibe-coded — precision-engineered in a domain
where AI fails."

**Presenter notes**: "While other companies in the AI governance space are
raising record rounds on pitch decks and prototypes, Anvil is a production-grade
system. The Rust kernel, the policy engine, the semantic graph, the authorship
attribution — all built. Precision-engineered in a domain where AI struggles:
deterministic analysis, sub-50-millisecond checks, repeatable results. We have 5
pilot teams today and developer influencers lined up to demo ahead of launch.
We're targeting 5,000 on the waitlist and 10 to 15 pilot teams by the time we
close this round. The product plays in the exact space AI fails at — precision —
and that's the point."

**Design note**: Metric cards with numbers, not screenshots. The product demo on
slide 6 has already shown the TUI — this slide should feel like a progress
dashboard. Clearly distinguish "built today" (confirmed) from "launch
trajectory" (targets) — investors penalise projections dressed as facts.

---

### Slide 11: Team

**Headline**: 25 years building what enterprises buy

**Structure**:

**Founder**:

- Joshua Boys, Founder & CEO
- Former Microsoft Azure Lead, Australia
- CEO of Arkahna — platform engineering for 100+ SaaS companies over 5 years
- 25+ years building enterprise software, leading teams, shipping SaaS

**Unfair advantage callout**: "Built governance tooling from inside the
enterprise buying process — not from a research lab"

**Advisory bench**:

- Senior advisors across enterprise software, startup scaling, and large SaaS
- Name advisors on the slide if they consent; otherwise present as a text block
  ("Advisory support across enterprise, SaaS, and compliance") — avoid unnamed
  silhouette placeholders which imply hidden identities

**Why solo founder = strength**:

- Capital efficient: £0 raised, production-grade product delivered
- No AI inside: this product required precision engineering, not prompt chaining
- Arkahna's 100+ SaaS client base provides direct market insight and
  distribution
- First hires are engineering + CRO — the team scales with the raise

**Closing beat**: "Built to make you trust your AI more — not by asking you to
trust ours."

**Presenter notes**: "I've spent 25 years in enterprise software — the last five
as CEO of Arkahna, a platform engineering company that works with over 100 SaaS
companies. I was the Azure lead in Australia for Microsoft. I know how
enterprises buy developer tools, because I've been on both sides of that
transaction. Anvil exists because I've watched AI coding tools arrive in my
clients' organisations with zero governance. The advisory bench includes senior
operators from enterprise, startups, and large SaaS. The team scales with this
raise — first hires are engineers and a CRO."

**Design note**: Single founder portrait left, credentials right. Advisory logos
or silhouettes below. No grid of empty headshots — lean into the "one person
built this" as a capital efficiency signal.

---

### Slide 12: The Ask

**Headline**: Own the category before the window closes

**The Raise**:

- £3–5M seed round
- £15–25M pre-money valuation
- Category: AI governance — hottest new category in developer tooling

**Use of Funds** (visual split):

- **Engineering** (~40%): 3–4 hires. Scale Rust kernel, platform layer
  (dashboard, multi-tenant policy management), ecosystem integrations (IDE,
  CI/CD)
- **Go-to-Market** (~30%): Enterprise-focused CRO + developer
  advocacy/community. Land-and-expand through Arkahna's 100+ SaaS network
- **Strategic Acquisition** (~20%): Acquire platform engineering IP from Arkahna
  and potentially others — clean arm's-length transactions, independently
  valued. Accelerates maturity and scale by absorbing proven infrastructure
  rather than rebuilding (months saved, not cost arbitrage)
- **Operations** (~10%): Compliance certification, infrastructure

**Milestones this round delivers**:

- Profitability and self-sufficiency on phase 1 (code governance)
- 5,000+ waitlist, 50+ paying teams, enterprise contracts
- Platform and ecosystem expansion complete
- Phase 2 ready: expand from developer governance to **knowledge worker
  governance** — without further dilution

**Data callout**: "EU AI Act enforcement: August 2026. 5 months to capture the
compliance purchasing wave."

**Presenter notes**: "We're raising £3–5M to own this category before the
compliance window closes. 40% goes to engineering — scaling the Rust kernel,
building the platform layer, and ecosystem integrations. 30% to go-to-market —
an enterprise CRO and developer advocacy to drive bottom-up adoption. 20% to
strategic acquisition of platform engineering IP — proven infrastructure we can
absorb rather than rebuild, accelerating our maturity by months. This round gets
us to profitability on code governance. That's phase 1. Phase 2 is the bigger
thesis: AI governance for all knowledge work — legal, finance, operations —
starting from the beachhead where the pain is sharpest and the tooling is most
mature. We reach phase 2 self-funded. No further dilution required."

**Design note**: Use of funds as a clean donut or horizontal bar. Milestones as
a timeline with the August 2026 EU AI Act deadline marked. Phase 2 vision as a
single line at the bottom — hint at scale, don't over-explain.

---

## Updates to Existing Slides

### Slide 6: Product — Headline Update

**Old**: "Built in Rust. Runs in your terminal." **New**: "Built in Rust. No AI
inside. 50ms per check."

Updated presenter notes should include: "Every check is deterministic —
programmatic, mechanical, repeatable. No AI reviewing AI. The same input always
produces the same output. This product plays in the exact space AI struggles
with: precision. And it runs in under 50 milliseconds."

---

## Investor Context

### Valuation Justification (£15–25M pre-money)

- Hottest new category in developer tooling — record pre-seed rounds being
  raised in the AI governance space establish category-level valuation precedent
- First-mover with production-grade product (most competitors are pre-product) —
  product completeness de-risks execution, which commands premium over
  pitch-stage peers
- Regulatory forcing function (EU AI Act) creates mandatory spend with a known
  deadline — this is not speculative demand
- Solo founder capital efficiency — £0 raised to date, production product built
  — demonstrates exceptional capital-to-output ratio
- Arkahna distribution channel (100+ SaaS clients) — built-in early market
  access that would take years to build from scratch

**Investor FAQ addition needed**: Add a valuation rationale question to
`plans/pitch-deck/content/investor-faq.md` addressing "why £15–25M pre-money for
a pre-revenue company" — anchor to category comps and product maturity delta.

### Anticipated Objections

1. **"Solo founder risk"** → Capital efficiency proof. Product exists. Team
   scales with raise. Advisory bench provides coverage.
2. **"Pre-revenue"** → Category is pre-revenue industry-wide. Product maturity
   is the differentiator. Pilots in progress.
3. **"Why not just add this to existing tools?"** → Requires fundamental
   re-architecture. Pre-commit + deterministic is a structural position, not a
   feature. See competitive matrix (slide 8).
4. **"Arkahna relationship"** → Fully independent companies. Arkahna is a
   distribution asset, not a dependency. Acquisition of IP is clean and
   accelerates maturity.

---

## Relationship to Existing Pipeline Materials

This spec completes the gaps in the orchestrator pipeline output. The 16
documents in `plans/pitch-deck/` remain the primary content source. This spec
adds:

- Completed copy for slides 10, 11, 12
- Updated headline for slide 6
- "No AI inside" threading across slides 6, 10, 11
- Investor context (valuation justification, objection handling)
- Strategic acquisition as a use-of-funds category

The `content/slide-copy.md` and `strategy/slide-outline.md` files should be
updated to reflect these decisions when the deck is produced.
