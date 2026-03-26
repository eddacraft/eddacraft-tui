# Narrative Arc: Anvil Pitch Deck

## Three-Act Structure

---

## Act I -- The Problem (Slides 1-3)

### Emotional Journey: Concern to Alarm

**Beat 1: The New Reality** (Slide 1 -- Title)

- AI is writing nearly half of all production code
- This is not experimental -- 90% of Fortune 100 use AI coding tools
- The productivity gains are real, but the governance question is unanswered

**Beat 2: The Governance Gap** (Slide 2 -- Problem)

- AI-generated code produces 1.7x more defects, 1.4x more critical issues
- Fewer than 50% of developers review AI code before committing
- 45% of AI code fails security tests (Veracode, 100+ models tested)
- Code duplication has increased 4x; refactoring has collapsed from 25% to <10%
- No tool governs code at the point of generation -- every tool scans after
  commit

**Beat 3: The Clock Is Ticking** (Slide 3 -- Why Now)

- EU AI Act high-risk requirements enforceable August 2026 (5 months)
- Penalties: up to 7% of global annual turnover
- Gartner: 40% of AI coding projects cancelled by 2027 due to escalating costs
- AI governance platform spend: USD 492M (2026), >USD 1B (2030)
- The window between "optional governance" and "mandatory compliance" is closing

### Key Question the Audience Should Be Asking

> "How do engineering teams maintain quality, compliance, and architectural
> integrity when AI is generating half the code?"

---

## Act II -- The Solution (Slides 4-6)

### Emotional Journey: Relief to Conviction

**Beat 4: Introducing Anvil** (Slide 4 -- Solution)

- Deterministic policy engine that governs AI workflows at file save
- Not AI reviewing AI -- policy-as-code (OPA/Rego) for predictable, auditable
  governance
- Line-level authorship attribution: human, AI, mixed, unknown
- Architecture drift detection via persistent semantic graph
- The "adult in the room" -- a flight recorder, not a chat app

**Beat 5: How It Works** (Slide 5 -- How It Works)

- File save triggers analysis (synchronous with generation)
- Policy evaluation against team-defined rules (OPA/Rego)
- Authorship classification at the line level
- Architecture graph update (dependency, trust, plan graphs)
- Structured governance events emitted (warnings, blocks, audit log)
- Flow:
  `File Save -> Parse -> Attribute -> Evaluate Policy -> Emit Governance Event`

**Beat 6: See It in Action** (Slide 6 -- Product Demo)

- Terminal UI (Ratatui/Rust) -- the product looks like the brand
- Real-time watcher showing policy enforcement live
- CLI output demonstrating authorship attribution
- Architecture drift detection in action

### Key Statement the Audience Should Believe

> "This is the governance layer that should have existed from the start of the
> AI coding revolution."

---

## Act III -- The Transformed State (Slides 7-13)

### Emotional Journey: Conviction to Commitment

**Beat 7: The Market Is Ready** (Slide 7 -- Market Opportunity)

- TAM: USD 21.5B across AI code tools + AppSec + governance
- SAM: USD 1.5-2.0B (governance/quality for AI-assisted development)
- AI governance platforms: USD 492M (2026), >USD 1B (2030)
- Regulatory pressure creates compliance forcing function

**Beat 8: Anvil Stands Alone** (Slide 8 -- Competitive Landscape)

- Positioning matrix: Anvil is the only tool that is both deterministic AND
  pre-commit
- Static analysis tools (SonarQube, Semgrep) are post-commit
- AI code review (CodeRabbit, Sourcery) is probabilistic
- Supply chain (Snyk, Socket) covers dependencies, not code provenance
- No competitor provides authorship attribution

**Beat 9: The Business** (Slide 9 -- Business Model)

- Developer-led adoption (bottom-up, CLI/editor integration)
- Per-seat pricing with enterprise tiers
- Land-and-expand: start with one team, expand to organisation
- Policy packs as expansion revenue (compliance-specific rule sets)

**Beat 10: Traction** (Slide 10 -- Traction / Validation)

- [EVIDENCE NEEDED: waitlist numbers, design partners, open source community
  metrics]
- Product maturity: Rust kernel, Ratatui TUI, OPA/Rego integration, authorship
  attribution built
- Technical validation: persistent semantic graph operational

**Beat 11: The Team** (Slide 11 -- Team)

- [EVIDENCE NEEDED: team bios, relevant expertise, advisors]

**Beat 12: The Ask** (Slide 12 -- The Ask)

- [EVIDENCE NEEDED: funding amount, use of funds, milestones]

**Beat 13: Appendix** (Slide 13 -- Appendix)

- Technical architecture deep dive
- Detailed competitive analysis
- Regulatory timeline
- Financial model assumptions

### Key Commitment the Audience Should Make

> "This team has built the right product at the right time for a market that is
> about to become mandatory. I want to be part of it."

---

## Narrative Design Principles

1. **Problem-first**: Build the case for governance before introducing Anvil.
   The problem sells the solution.
2. **Data-driven**: Every claim is backed by research. No empty adjectives.
3. **Show the gap**: The positioning matrix (slide 8) is the visual proof that
   no one else occupies this space.
4. **Regulatory urgency**: August 2026 is not abstract -- it is 5 months away.
   This creates purchasing urgency.
5. **Technical credibility**: The product demo (slide 6) proves this is built,
   not vapourware.
6. **Brand consistency**: The deck should feel like the product -- dark,
   precise, sharp, terminal aesthetic.

---

## Pacing Notes

| Slide             | Time    | Purpose                                               |
| ----------------- | ------- | ----------------------------------------------------- |
| 1. Title          | 30s     | Set the stage                                         |
| 2. Problem        | 2 min   | Build tension -- the quality crisis is real           |
| 3. Why Now        | 1.5 min | Create urgency -- regulatory deadline + market timing |
| 4. Solution       | 2 min   | Relief -- there is an answer                          |
| 5. How It Works   | 1.5 min | Technical credibility                                 |
| 6. Demo           | 2 min   | Proof it works                                        |
| 7. Market         | 1 min   | Scale of opportunity                                  |
| 8. Competitive    | 1.5 min | Defensibility                                         |
| 9. Business Model | 1 min   | Path to revenue                                       |
| 10. Traction      | 1 min   | Momentum                                              |
| 11. Team          | 30s     | Trust                                                 |
| 12. Ask           | 1 min   | Call to action                                        |
| Total             | ~15 min | Standard pitch length                                 |
