# Win Themes: Anvil by EddaCraft

## Overview

Five win themes derived from Phase 1 research. Each theme names a specific buyer
challenge, connects it to a concrete Anvil capability, and is backed by
quantified evidence. These themes are Anvil-specific -- swapping the product
name would break them.

---

## Theme 1: "Governance at the speed of generation"

### Buyer Need

Engineering leaders need governance that operates at the same speed as AI code
generation. Current tools scan after commit or at PR time -- by then, ungoverned
code is already in the codebase.

### Anvil Differentiator

Anvil enforces policy at file save, the moment code is generated. No other tool
operates at this point in the workflow. The governance layer is synchronous with
generation, not a separate post-hoc step.

### Proof Points

- 46% of code is AI-generated (GitHub/GitClear 2025) -- this volume cannot be
  manually reviewed
- Fewer than 50% of developers review AI code before committing (Stack
  Overflow 2025)
- Post-commit scanning finds defects after the cost of fixing has increased
  10-100x

### Appears in Slides

2 (Problem), 4 (Solution), 5 (How It Works)

---

## Theme 2: "Deterministic rules for probabilistic outputs"

### Buyer Need

CTOs and security leaders cannot accept AI reviewing AI -- it compounds
uncertainty. They need deterministic, auditable governance that does not
introduce its own probabilistic risk.

### Anvil Differentiator

Anvil uses policy-as-code (OPA/Rego) for deterministic analysis. It does not use
AI to review AI-generated code. The governance layer is predictable,
reproducible, and auditable -- the same input always produces the same output.

### Proof Points

- Best AI model produces secure code only 56% of the time (BaxBench 2025)
- AI code review tools are probabilistic -- CodeRabbit, Sourcery, Qodo suggest
  but cannot guarantee
- Gartner: 2,500% software defect increase projected from prompt-to-app
  (by 2028)

### Appears in Slides

2 (Problem), 4 (Solution), 8 (Competitive Landscape)

---

## Theme 3: "Every line has an author"

### Buyer Need

Compliance teams need to know who or what wrote each line of production code.
Regulations (EU AI Act), audit frameworks (SOC 2), and insurance underwriters
increasingly require provenance. No tool currently provides line-level
authorship in production codebases.

### Anvil Differentiator

Anvil provides line-level authorship attribution: human, AI, mixed, or unknown.
This creates an auditable provenance chain from generation through deployment.
It is not metadata about commits -- it is classification of every line.

### Proof Points

- EU AI Act (August 2026): requires documentation and traceability of AI systems
- Penalties up to 7% of global turnover for non-compliance
- AI-generated code causes 1 in 5 breaches (Aikido Security 2026) -- attribution
  is the first step to accountability
- No competitor offers this capability

### Appears in Slides

3 (Why Now), 5 (How It Works), 7 (Market Opportunity), 13 (Appendix)

---

## Theme 4: "Architecture drift is the silent killer"

### Buyer Need

Engineering teams accumulate architecture drift invisibly. AI-generated code
accelerates this -- it optimises for local correctness, not structural
integrity. By the time drift is detected, the codebase is already degraded.

### Anvil Differentiator

Anvil maintains a persistent semantic graph of the repository (dependency,
trust, plan graphs) and detects architecture drift incrementally. It does not
just scan for violations -- it tracks trajectory. "You are trending toward
structural instability" is a fundamentally different signal than "you broke a
rule."

### Proof Points

- Code duplication increased 4x with AI assistance (GitClear 2025)
- Refactoring dropped from 25% to <10% of changed lines (GitClear 2025)
- Copy/paste exceeded refactoring for the first time in 2024
- 1.64x more maintainability errors in AI code (CodeRabbit)
- 75% of tech decision-makers face moderate to severe AI technical debt (2026
  survey)

### Proof Points

- Year 2 maintenance costs reach 4x traditional levels for unmanaged AI code
- 40% of AI-augmented coding projects cancelled by 2027 (Gartner)

### Appears in Slides

2 (Problem), 4 (Solution), 5 (How It Works), 8 (Competitive Landscape)

---

## Theme 5: "Compliance is continuous, not quarterly"

### Buyer Need

Enterprise compliance today is periodic -- quarterly audits, annual reviews.
With AI generating 46% of code and regulatory deadlines approaching (EU AI Act
August 2026), periodic compliance is insufficient. Organisations need continuous
assurance.

### Anvil Differentiator

Anvil provides continuous governance at file save. Every code change is
policy-checked, attributed, and logged. The audit trail is real-time, not
reconstructed. Compliance evidence is generated as a by-product of development,
not as a separate exercise.

### Proof Points

- EU AI Act high-risk deadline: August 2026 (5 months away)
- Gartner: AI governance spend USD 492M (2026), >USD 1B (2030)
- Organisations with governance platforms are 3.4x more effective (Gartner
  survey, 360 orgs)
- Governance reduces regulatory expenses by 20% (Gartner)

### Appears in Slides

3 (Why Now), 7 (Market Opportunity), 9 (Business Model)

---

## Win Theme Matrix

| Theme                                         | Buyer Need                        | Differentiator                                       | Key Proof Point                         | Slides     |
| --------------------------------------------- | --------------------------------- | ---------------------------------------------------- | --------------------------------------- | ---------- |
| Governance at the speed of generation         | Governance must match AI velocity | File-save enforcement                                | 46% code is AI-generated, <50% reviewed | 2, 4, 5    |
| Deterministic rules for probabilistic outputs | No AI-on-AI trust compounding     | OPA/Rego policy-as-code                              | Best AI model: 56% secure code          | 2, 4, 8    |
| Every line has an author                      | Provenance for compliance/audit   | Line-level attribution (human/AI/mixed/unknown)      | EU AI Act Aug 2026, 7% turnover penalty | 3, 5, 7    |
| Architecture drift is the silent killer       | Invisible structural decay        | Persistent semantic graph, trajectory analysis       | 4x code cloning, refactoring <10%       | 2, 4, 5, 8 |
| Compliance is continuous, not quarterly       | Periodic compliance cannot scale  | Real-time audit trail, continuous policy enforcement | 3.4x governance effectiveness (Gartner) | 3, 7, 9    |
