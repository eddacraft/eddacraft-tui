# Public Site Decision Integrity Redesign

**Status:** Approved 2026-08-17

## Purpose

Redesign `eddacraft.ai` as a product-led public site for `anvil` that connects the shipping generation-time control point to the larger Decision Integrity system without presenting future capabilities as current.

The site must explain why `anvil` matters, prove that valuable functionality operates today and convert technical visitors into early-access users.

## Authorities

- Product truth: this repository.
- External positioning: `eddacraft/eddacraft-gtm/positioning/Positioning.md`.
- Visual execution: `eddacraft/brand-and-design`.
- Investor-funded destination: contextual input only, never current product truth.

## Narrative principle

Use one continuous public story with one explicit delivery boundary. Do not distribute roadmap badges across the entire page.

Flow:

`customer problem → working product → larger system → honest delivery boundary → company vision`

## Information architecture

### Navigation

Show `eddacraft` as the company and `anvil` as the active product. Retain access to documentation, security, early access and waitlist conversion.

### Hero

The hero makes current, supportable claims only.

Approved direction:

```text
// GENERATION_TIME_TRUST

TRUST THE CODE
YOUR AI WRITES.

anvil is the independent, deterministic control point
for AI-assisted software engineering.

Understand the change. Apply your standards.
Stop unsafe work before it reaches review.
```

Retain installation, early-access and documentation actions.

Update the terminal demonstration to show a concrete interception, context, policy, block/pass and current receipt/provenance flow. Every line must represent current behaviour.

### Shipping proof

Place a narrow operational proof strip directly below the hero. Candidate facts include local execution, deterministic evaluation, graph context, agent-agnostic operation, current release and measured latency. Exact values must be verified immediately before implementation.

### Trust gap

Headline direction:

```text
AI CAN CREATE MORE
THAN HUMANS CAN REVIEW.
```

Distinguish:

- logs: what happened;
- evidence: what was true;
- policy: what was required;
- receipts: why an action was trusted.

### Current-to-destination bridge

Headline direction:

```text
PROTECTION IS THE ENTRY POINT.
DECISION INTEGRITY IS THE SYSTEM AROUND IT.
```

Explain that current protection and software understanding form the foundation for the larger intent, evidence, policy and receipt system.

### Canonical flywheel

Render `UNDERSTAND → BUILD → DECIDE → LEARN` as accessible semantic HTML and SVG.

Use one quiet visual distinction:

- solid ember path for foundations operating today;
- Structure-grey path for the system being completed;
- one concise legend.

On narrow screens, convert the diagram into a vertical sequence with equivalent reading order. The diagram remains understandable without animation or colour.

### Four-stage product architecture

Replace the arbitrary six-feature grid with:

- Understand: software, dependencies, ownership and intent.
- Build: context, impact analysis, planning, review and explanation.
- Decide: interception, deterministic policy, block or pass.
- Learn: receipts, drift and outcomes improving future understanding.

Current capabilities render normally. Directional capabilities use muted treatment. “Build” means supporting humans and agents with context; `anvil` is not presented as the coding agent.

### Single delivery boundary

Use one explicit status section.

```text
THE CONTROL POINT SHIPS TODAY.
THE TRUST CHAIN COMES NEXT.
```

Operating today is sourced from current product truth. The system being completed may include general intent conformance, evidence providers, independently verifiable receipts, drift and outcome learning only when accurately framed.

### Decision model and independence

Explain:

`INTENT + EVIDENCE + POLICY → DETERMINISTIC DECISION → DECISION RECEIPT`

State why the system creating work should not be the sole authority deciding whether that work is trustworthy.

### Company band

Near the end:

```text
// BUILT_BY_EDDACRAFT

TRUST INFRASTRUCTURE
FOR AI-ASSISTED WORK.
```

Explain that `anvil` begins with software engineering while `eddacraft` has the broader mission of making AI-assisted work independently trustworthy.

### Conversion

Retain early-access, installation, documentation and waitlist flows. The redesign must not regress the existing access service.

## Visual system

Follow Nordic Terminal exactly:

- canonical tokens from `brand-and-design`;
- Void surfaces with Structure fills and borders;
- JetBrains Mono for system voice;
- Inter for prose;
- bracket iconography for context, action and history;
- lowercase `eddacraft` and `anvil`;
- British English;
- square corners;
- no gradients, shadows, metaphorical icons or stock imagery;
- restrained cursor or path motion only;
- full reduced-motion behaviour.

Retain Inter as the canonical narrative voice and restrict the active surface to the five colours defined by `brand-and-design`.

## Technical design

Remain within the existing Next.js 16, React 19 and Tailwind CSS 4 application.

Create focused, server-renderable content sections where possible. Use client components only for existing interaction, terminal motion and deliberately interactive diagram behaviour.

Update page metadata, Open Graph and Twitter assets so the old write-gate headline does not survive in social previews.

Preserve the early-access API, privacy page, security page, Vercel deployment structure and current analytics.

## Accessibility and responsive behaviour

- Semantic heading order.
- Keyboard-visible focus using approved border colours.
- Text alternatives for diagrams.
- No information conveyed by colour alone.
- Vertical diagram fallback for narrow screens.
- Respect `prefers-reduced-motion`.
- Preserve readable body line lengths and touch targets.
- Verify at representative mobile, tablet and desktop widths.

## Non-goals

- Implementing future product functionality.
- Building a separate corporate site or `/anvil` route.
- Publishing a detailed roadmap.
- Changing authentication, waitlist or deployment architecture.
- Changing the canonical brand.

## Acceptance criteria

- Visitors understand the current customer outcome before the category architecture.
- All hero and proof claims are verifiable in this repository.
- The Decision Integrity direction is clear without appearing shipped.
- The flywheel and four-stage architecture are accessible and responsive.
- `eddacraft` and `anvil` are always lowercase in prose.
- The site passes repository checks and production build.
- Early-access and waitlist flows remain functional.
- Metadata and social assets use the approved positioning.
- The site passes the `brand-and-design` governance diff.
