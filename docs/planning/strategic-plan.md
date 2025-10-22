# Anvil Strategic Plan

## Executive Summary

**Mission**: Build the AI-governed development platform that makes prototype →
production safe through validated, deterministic plans.

**Current Phase**: Act 1 (Developer Wedge) - MVP **Timeline**: 12-14 weeks to
MVP (SpecKit adapter complete ✅), 12 months to Product-Market Fit **Vision**:
Three-act expansion from dev tools → knowledge work → enterprise platform

---

## The Three-Act Vision

### Act 1: The Wedge (MVP — Developers) [Current]

**The Pain Today**: Developers are experimenting with AI everywhere, but outputs
are messy, ungoverned, and impossible to take safely into production.
"Vibe-coded" repositories are brittle and untrustworthy.

**Anvil's Answer**: A sidecar runtime + deterministic plan specification. Every
AI/human intent becomes a reproducible plan, validated through a single gate,
and applied with provenance and rollback.

**Positioning**: _"Anvil is the AI-governed development sidecar that makes
prototype → production safe."_

**Why Investors Buy This**: The developer tools market is huge and highly
visible. It gives us a sticky entry point and early credibility.

**Success Metrics**:

- 50+ development teams using Anvil in production
- NPS >50 with early adopters
- $500K+ ARR within 12 months
- GitHub stars >1000, strong community engagement

### Act 2: The Expansion (Beyond Devs) [12-18 Months]

**The Observation**: The same cycle — intent → plan → validate → apply → govern
— isn't unique to code. Other teams struggle with the same thing: messy
AI-generated drafts, lack of reproducibility, no provenance, governance
headaches.

**The Opportunity**:

- **Management consultants** generating strategy decks
- **Financial analysts** creating forecast models
- **Legal teams** drafting contracts
- **Policy teams** writing regulations
- **Researchers** producing papers

**The Same Workflow**:

```
Intent → Plan → Validate → Apply → Govern
```

**Examples**:

**Consultants**:

```bash
anvil plan "Market entry strategy for Southeast Asia"
anvil gate strategy-deck.pptx  # Validates: citations, data sources, logic
anvil apply strategy-deck.pptx  # Publishes with full provenance
```

**Financial Analysts**:

```bash
anvil plan "Q4 revenue forecast model"
anvil gate forecast-model.xlsx  # Validates: formulas, assumptions, data
anvil apply forecast-model.xlsx  # Publishes with audit trail
```

**Positioning**: _"Anvil is how teams safely co-work with AI on critical
deliverables."_

**Why This Matters**: Act 2 expands TAM 10x (from 30M developers to 300M+
knowledge workers).

**Success Criteria for Expansion**:

- ✅ 50+ developer teams using Anvil successfully
- ✅ Strong product-market fit (NPS >50)
- ✅ Market credibility established
- ✅ Customers explicitly asking "can this work for our analysts/consultants?"

**Strategic Timing**: Only pursue Act 2 when Act 1 is proven. Credibility from
developer adoption is essential for knowledge worker expansion.

### Act 3: The Platform (Horizontal Layer) [24+ Months]

**The Vision**: Anvil becomes the horizontal AI governance layer for
enterprises. Every AI interaction — code, documents, analysis, strategy — flows
through Anvil's validation and audit system.

**The Platform**:

```text
Every AI interaction in the enterprise flows through Anvil:
- Developers: Governed code
- Analysts: Governed models
- Consultants: Governed strategy
- Legal: Governed contracts
- Operations: Governed processes
```

**Positioning**: _"We're the horizontal AI governance layer. Every enterprise
needs this."_

**Strategic Integrations**:

- Deep partnerships with AI providers (Anthropic, OpenAI)
- Enterprise tool integrations (Confluence, ServiceNow, SAP)
- Compliance frameworks (SOC2, ISO, regulatory)

**Business Model**: Enterprise-wide deployment with platform pricing.

**Why This Is The Endgame**:

- Cross-functional (all departments)
- Mission-critical (all AI work)
- Defensible (massive switching costs)
- Platform economics (network effects)

---

## Strategic Positioning

### What to Say to Different Audiences

#### To Developers (Public Pitch) — Act 1

_"Anvil makes AI coding production-ready._

_AI tools ship code fast, but getting it to production safely still requires
manual review. Anvil automates that._

_Every change becomes a validated plan. Plans run through quality gates. Code
that passes ships. Code that fails rolls back._

_Works with Cursor, Copilot, Claude Code, or whatever you use. One pipeline for
all AI-generated code._

_Anvil: Ship at AI speed. Sleep at human peace."_

**No mention of Act 2/3.** Focused wedge.

#### To Investors (Private Vision) — Full Picture

_"We're starting with developers because they adopt new tools fastest and have
the most urgent governance pain._

_But the same workflow — intent, plan, validate, apply, govern — applies to all
knowledge work._

_Consultants generate strategy decks with AI. Financial analysts generate
models. Legal teams generate contracts. All need the same thing: validation,
provenance, rollback._

_Act 1 gets us revenue and credibility. Act 2 expands TAM 10x. Act 3 makes us
platform infrastructure._

_We're not building a dev tool. We're building the governed AI layer for
enterprises."_

**Full three-act vision.** Show the ambition.

#### To Enterprise Buyers (Act 2) — Future

_"Your teams are already using AI everywhere. We make it governed, auditable,
and safe. Started with dev, now expanding to all critical work."_

**Hook:** Credibility from Act 1, expansion story.

#### To Strategic Partners (Act 3) — Future

_"We're the horizontal AI governance layer. Every enterprise needs this. Let's
integrate."_

**Hook:** Platform play, ecosystem.

---

## Technical Architecture

### Core Concept: The Anvil Plan Spec (APS)

**The Insight**: APS is our moat. It's a standardised, hash-stable format for AI
intent that makes everything else possible.

**APS Characteristics**:

- **Deterministic**: Same intent → same hash (reproducible)
- **Validated**: Schema-enforced structure (Zod + JSON Schema)
- **Auditable**: Immutable evidence trail (provenance)
- **Reversible**: Full rollback capability (safety)

**Critical Architectural Decision**: APS is **internal only**. Users never see
it unless they want to.

### Interoperability Strategy (The Real Wedge)

**The Problem We Solved**: Users won't adopt a new planning format. They already
have SpecKit, BMAD, ADRs, PRDs, etc.

**Our Solution**: Work with **everything**. Anvil reads and writes existing
formats, uses APS internally for validation and execution.

**Architecture**:

```
SpecKit/BMAD/User Format
         ↓
    [Adapter] (converts to APS)
         ↓
    APS (internal)
         ↓
    [Validation & Execution]
         ↓
    [Adapter] (converts back)
         ↓
SpecKit/BMAD/User Format (updated with evidence)
```

**Adapters Are The Wedge**: This is what makes Anvil adoptable. Users keep their
existing workflows; Anvil just makes them safer.

### System Architecture

```text
┌─────────────────────────────────────────────┐
│         User Formats (External)             │
│  • SpecKit (spec.md, plan.md)               │
│  • BMAD (PRD, architecture docs)            │
│  • Future: ADRs, Confluence, Notion         │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│      Format Adapters (Interoperability)     │
│  • Auto-detect format                        │
│  • Parse to APS (internal)                   │
│  • Serialise back to original format         │
│  • Preserve formatting and structure         │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│         APS Core (Internal Format)           │
│  • Hash-stable schema (Zod)                  │
│  • Deterministic canonicalisation            │
│  • Schema validation                         │
│  • TypeScript type generation                │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│         Plan Gate (Validation Layer)         │
│  • Lint checks (ESLint)                      │
│  • Test execution (Vitest)                   │
│  • Coverage analysis                         │
│  • Secret scanning                           │
│  • Policy enforcement (OPA/Rego)             │
│  • Evidence collection                       │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│      Sidecar (Execution Runtime)             │
│  • Dry-run (preview changes)                 │
│  • Apply (execute changes)                   │
│  • Rollback (revert changes)                 │
│  • Audit trail generation                    │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│    Evidence & Provenance (Audit Layer)       │
│  • Immutable evidence bundles                │
│  • Full audit trail                          │
│  • Provenance tracking                       │
│  • Rollback snapshots                        │
└─────────────────────────────────────────────┘
```

### Tech Stack

**Language**: TypeScript everywhere (Node.js runtime)

- Future: Go/Rust for performance-critical components (post-MVP)

**Schema & Validation**:

- Zod for runtime validation and TypeScript types
- JSON Schema for interoperability
- Ajv for JSON Schema validation

**Policy Engine**: OPA (Open Policy Agent) with Rego

- Bundled binary, versioned with repo
- Policy-as-code approach

**CLI Framework**: Commander.js

- Cross-platform support (Linux, macOS, Windows)
- Professional CLI experience

**Testing**:

- Vitest for unit and integration tests
- Fixture-based testing for adapters
- Golden tests for hash determinism

**Build System**: Nx monorepo

- pnpm for package management
- TypeScript strict mode
- ESLint + Prettier for code quality

---

## MVP Roadmap (16 Weeks)

### Critical Path

The fastest route to a fundable, demonstrable product:

**Weeks 1-2: Complete APS Core** ✅ COMPLETE

- ✅ Finish remaining 20% of APS integration
- ✅ Export all utilities from core package
- ✅ Documentation for developers
- **Completed**: October 13, 2025

**Weeks 3-4: SpecKit Adapter (Customer #1)** ✅ COMPLETE

- ✅ Parser and serialiser for SpecKit format (~2,469 LOC)
- ✅ Support for spec.md, plan.md, and tasks.md
- ✅ Round-trip conversion with evidence injection
- ✅ Comprehensive test suite (51 tests, 49 passing)
- ✅ Registry integration with auto-detection
- **Completed**: October 14, 2025
- **Next**: Demo with Customer #1

**Weeks 5-6: BMAD Adapter (Customer #2)** ⏳ IN PROGRESS

- Parser and serialiser for BMAD/PRD format
- Requirements extraction (REQ-XXX format)
- Round-trip conversion with evidence injection
- Demo with second customer
- **Target**: Week 5-6

**Weeks 7: CLI Integration** ⏳ NEXT

- Wire CLI commands to adapter auto-detection
- Evidence updates work across formats
- Configuration file support
- **Milestone**: Both customers can validate plans in their formats

**Weeks 9-10: Policy Engine**

- OPA binary integration
- Sample policies (coverage, flag risk, change scope)
- Policy CLI commands

**Weeks 11-12: Dry-run**

- Preview system for changes
- Diff generation
- CLI command with syntax highlighting
- **Milestone**: "Wow moment" feature complete

**Weeks 13-14: Apply**

- Transactional application of changes
- Snapshot creation
- Audit trail generation
- CLI command with safety guards
- **Milestone**: Changes can be applied safely

**Weeks 15-16: Rollback & GitHub Action**

- Rollback implementation
- GitHub Action for PR validation
- **Milestone**: Complete end-to-end workflow

### MVP Success Criteria

We're "done" with MVP when:

1. ✅ A developer can run:

   ```bash
   anvil gate spec.md        # Works with SpecKit
   anvil gate prd.md         # Works with BMAD
   anvil dry-run spec.md     # Shows preview
   anvil apply spec.md       # Applies safely
   anvil rollback <plan-id>  # Reverts changes
   ```

2. ✅ Plans validate through quality gates:
   - Lint checks pass
   - Tests pass
   - Coverage meets threshold
   - No secrets detected
   - Policies enforced

3. ✅ GitHub Action blocks PRs that fail validation

4. ✅ Evidence is immutable and auditable

5. ✅ 15-20 pilot teams using Anvil successfully

---

## Go-to-Market Strategy

### Act 1: Developer Adoption (Months 1-12)

**Distribution Channels**:

1. **GitHub Action** - Easiest adoption path (add to workflow, it just works)
2. **CLI Tool** - For local development workflows
3. **VS Code Extension** - Future: Inline feedback (post-MVP)

**Target Segments**:

- **Early Adopters**: Startups using AI coding tools heavily (Cursor, Copilot)
- **Platform Teams**: Companies standardising AI development practices
- **DevOps/SRE**: Teams responsible for deployment safety

**Messaging**:

- "Stop manually reviewing AI-generated code"
- "Validated AI development that just works"
- "Ship at AI speed, sleep at human peace"

**Pricing** (Post-MVP):

- **Free Tier**: Open source CLI, unlimited local use
- **Team Tier**: $29/dev/month - GitHub integration, team policies, audit logs
- **Enterprise**: Custom - SSO, RBAC, compliance features

**Growth Tactics**:

1. **Launch Partners**: 2-3 high-profile companies using Anvil in production
2. **Community**: GitHub stars, blog posts, conference talks
3. **Content**: "AI coding safety" thought leadership
4. **Integration**: Partnerships with Cursor, Claude Code, etc.

### Act 2: Knowledge Worker Expansion (Months 12-24)

**Target Segments**:

- Management consulting firms
- Financial services analysts
- Legal teams
- Policy/government teams

**Distribution**:

- Enterprise sales (top-down)
- Strategic partnerships (consulting firms, enterprise software)

**Messaging**:

- "Validated AI collaboration for critical work"
- "From code to strategy, safely"
- "Enterprise AI governance"

### Act 3: Platform Play (Months 24+)

**Strategy**: Become mission-critical infrastructure. Every AI interaction in
the enterprise flows through Anvil.

**Partnerships**:

- AI providers (Anthropic, OpenAI) - "verified by Anvil"
- Enterprise tools (Confluence, ServiceNow) - deep integrations
- Compliance frameworks (SOC2, ISO) - certified governance

---

## Competitive Landscape

### Direct Competitors (Act 1)

**Current State**: No direct competitors. We're creating a new category.

**Adjacent Tools**:

- **GitHub Copilot**: AI coding, no validation
- **Cursor**: AI IDE, no governance
- **Qodo (formerly CodiumAI)**: AI testing, not governance
- **Snyk / SonarQube**: Static analysis, not AI-specific

**Our Differentiation**: We're the only tool that makes AI development
deterministic, validated, and reversible as a complete system.

### Potential Competitors (Future)

**If We're Successful, Who Enters?**:

- **GitHub**: Could build governance into Copilot
- **Anthropic**: Could add validation to Claude Code
- **HashiCorp**: Could extend Terraform paradigm to code

**Our Moats**:

1. **First-mover advantage**: Category definition
2. **APS as standard**: Network effects from format adoption
3. **Adapter ecosystem**: Works with everything
4. **Enterprise relationships**: Once deployed, hard to replace
5. **Safety reputation**: Trust takes years to build

---

## Business Model

### Revenue Streams

**Act 1 (Developers)**:

- **Team Tier**: $29/dev/month
- **Enterprise**: Custom pricing (starts ~$50K/year)

**Act 2 (Knowledge Workers)**:

- **Professional**: $49/user/month
- **Enterprise**: Custom pricing (~$200K+/year)

**Act 3 (Platform)**:

- **Platform Tier**: $100K-$1M+/year (enterprise-wide)
- **Partnership Revenue**: Integration fees, revenue share

### Financial Projections (Illustrative)

**Year 1 (Act 1 MVP)**:

- 50 paying teams (avg 10 devs) = $174K ARR
- 5 enterprise deals (avg $50K) = $250K ARR
- **Total: $424K ARR**

**Year 2 (Act 1 Scale)**:

- 500 paying teams (avg 10 devs) = $1.74M ARR
- 50 enterprise deals (avg $75K) = $3.75M ARR
- **Total: $5.49M ARR**

**Year 3 (Act 2 Begin)**:

- Developer revenue: $10M ARR
- Knowledge worker revenue: $5M ARR
- **Total: $15M ARR**

**Year 4-5 (Act 3)**:

- Platform expansion: $50M+ ARR potential

---

## Success Metrics & KPIs

### Product Metrics (Act 1)

**Adoption**:

- GitHub Action installs
- CLI downloads
- Active teams (WAU/MAU)

**Engagement**:

- Plans validated per team per week
- Gate pass rate (target: >80%)
- Rollback frequency (expect <5% of applies)

**Quality**:

- False positive rate (target: <10%)
- User-reported bugs
- NPS score (target: >50)

**Growth**:

- Week-over-week growth in active teams
- Conversion rate: free → paid
- Expansion revenue (seats added)

### Business Metrics

**Revenue**:

- ARR and growth rate
- Logo acquisition rate
- Expansion revenue %

**Efficiency**:

- Customer Acquisition Cost (CAC)
- Lifetime Value (LTV)
- LTV:CAC ratio (target: >3:1)

**Market**:

- Share of voice in "AI coding safety"
- GitHub stars and community engagement
- Inbound lead flow

---

## Risk Mitigation

### Technical Risks

**Risk**: APS adoption is low (users reject another standard) **Mitigation**: ✅
Adapters strategy - we work with existing formats

**Risk**: Performance issues with large repositories **Mitigation**: Parallel
execution, caching, future Rust/Go rewrite

**Risk**: False positives in gate checks **Mitigation**: Configurable
thresholds, policy customisation, continuous improvement

### Market Risks

**Risk**: GitHub builds this into Copilot **Mitigation**: First-mover advantage,
enterprise relationships, adapter ecosystem

**Risk**: Developers reject "more process" **Mitigation**: Make it invisible
(GitHub Action), show clear ROI (time saved)

**Risk**: Enterprise sales cycle too long **Mitigation**: Bottom-up adoption
first, land-and-expand strategy

### Execution Risks

**Risk**: MVP takes longer than 16 weeks **Mitigation**: Ruthless scope
management, adapter-first approach, defer non-essential features

**Risk**: Can't find 15-20 pilot customers **Mitigation**: Launch partner
strategy, strong developer relations, community building

**Risk**: Team capacity constraints **Mitigation**: Phased hiring, prioritise
adapters and core validation

---

## Team & Hiring Plan

### MVP Team (Current)

**Core Team**: Founders + contractors

- 1-2 Full-stack engineers (TypeScript/Node.js)
- Product leadership
- Customer success/support

### Post-MVP (Months 3-6)

**Additions**:

- Senior Backend Engineer (sidecar, performance)
- DevRel / Developer Advocate
- Enterprise Sales (for Act 2 prep)

### Growth Phase (Months 6-12)

**Additions**:

- Engineering Manager
- 2-3 Additional Engineers
- Product Manager
- Customer Success Manager

---

## Investment & Fundraising

### Funding Requirements

**Pre-seed/Seed** (Current): $500K-$1.5M

- **Use**: MVP development, initial GTM, 15-20 pilot customers
- **Runway**: 12-18 months
- **Milestones**: MVP shipped, product-market fit signals, $500K ARR

**Series A** (12-18 months): $5-10M

- **Use**: Scale sales/marketing, Act 2 development, team growth
- **Runway**: 24 months
- **Milestones**: $5M ARR, 500+ teams, Act 2 launched

### Investor Pitch

**The Problem**: AI coding is fast but reckless. Teams can't trust AI changes in
production.

**The Solution**: Anvil makes AI development deterministic, like Terraform did
for infrastructure.

**The Market**: 30M developers today, 300M+ knowledge workers tomorrow.

**The Traction**: [15-20 pilot teams + launch partner]

**The Ask**: [$X] to ship MVP and get to product-market fit.

**The Vision**: Build the governed AI layer for enterprises (Act 1 → Act 2 → Act
3).

---

## What's Next: Execution Plan

### Immediate Actions (This Week) - **Week 5**

1. ✅ ~~Finish APS core~~ COMPLETE
2. ✅ ~~Ship SpecKit adapter~~ COMPLETE (2,469 LOC, 51 tests)
3. **Start BMAD adapter** development (Week 5-6 target)
4. **Customer #1 demo preparation** with SpecKit adapter

### Next 30 Days

1. **Ship BMAD adapter** with full test coverage
2. **CLI integration** for adapter auto-detection
3. **Demo with Customer #1 and #2** and get feedback
4. **Begin fundraising conversations** (if applicable)

### Next 60-90 Days (MVP Complete)

1. **Both adapters working** with full CLI and gate integration
2. **15-20 pilot customers** using Anvil
3. **Launch partner secured** (ideally)
4. **Fundraise closed** (if pursuing)
5. **MVP feature-complete** and stable

### Progress to Date (as of October 16, 2025)

✅ **Completed**:

- Phase 1: Foundations (100%)
- Phase 2: APS Core (100%)
- Phase 2.5: Adapter Framework (100%)
- Phase 2.5: SpecKit Adapter (100%)
- Phase 4: Gate v1 (100%)

⏳ **In Progress**:

- Phase 2.5: BMAD Adapter (0%)

📋 **Remaining**:

- Phase 3: CLI Integration with adapters
- Phase 5: Policy Engine (OPA/Rego)
- Phase 6: Sidecar & Dry-run
- Phase 7: Apply & Rollback
- Phase 8: GitHub Integration

**Overall Progress**: ~28% complete

---

## Appendix: Key Decisions Log

### Decision 1: APS as Internal Format

**Date**: 2025-09-30 **Decision**: APS is internal only; users work with their
existing formats **Rationale**: Users won't adopt another standard;
interoperability is the wedge **Impact**: Adapter development is now critical
path

### Decision 2: Adapters Before Features

**Date**: 2025-09-30 **Decision**: Ship SpecKit and BMAD adapters before packs,
productioniser, etc. **Rationale**: Adoption is blocked without
interoperability; features can wait **Impact**: Reprioritised roadmap, adapters
in Weeks 3-6

### Decision 3: Three-Act Vision

**Date**: 2025-09-29 **Decision**: Articulate full vision (Act 1 → 2 → 3) to
investors, focus Act 1 publicly **Rationale**: Shows ambition to investors,
avoids confusion with early customers **Impact**: Different messaging for
different audiences

### Decision 4: MVP Scope Cut

**Date**: 2025-09-30 **Decision**: Defer packs, productioniser heuristics, VS
Code extension, memory layer **Rationale**: Validation and safety are core;
features can come post-PMF **Impact**: Faster MVP timeline, clearer value
proposition

---

## Conclusion

Anvil is positioned to become the definitive platform for AI-governed
development and, ultimately, all AI-governed knowledge work. We have:

1. **A clear wedge** (Act 1: Developers)
2. **A massive expansion opportunity** (Act 2: Knowledge workers)
3. **A defensible endgame** (Act 3: Enterprise platform)
4. **A pragmatic MVP path** (16 weeks to fundable product)
5. **An interoperability strategy** (work with everything)

The question isn't whether validated AI development is needed — it clearly is.
The question is execution: Can we ship the MVP, find product-market fit, and
expand before competitors enter?

**We believe the answer is yes.** Let's build it.

---

**Document Version**: 2.0 (30 September 2025) **Authors**: Anvil Team **Next
Review**: Weekly during MVP development
