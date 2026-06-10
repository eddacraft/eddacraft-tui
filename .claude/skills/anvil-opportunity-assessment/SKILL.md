---
name: anvil-opportunity-assessment
description: Assess whether a repository, product, framework, paper, specification, or technical project should consume anvil roadmap space. Use when Morgan proposes a candidate, a strategically interesting GitHub project or dependency appears, a clean-room reimplementation or dependency-adoption decision is being weighed, or the user asks whether anvil should care about a technical opportunity.
---

# anvil opportunity assessment

## Purpose

Assess a repository, product, framework, paper, specification, or technical project and determine whether it deserves roadmap attention within anvil.

This is **not** a repository review skill.

This is **not** a dependency review skill.

This is a **product and architecture assessment** skill.

The objective is to identify valuable primitives, workflows, governance mechanisms, evidence models, operational patterns, developer experiences, architectural approaches, and customer-facing capabilities that could strengthen anvil.

Use UK/AU English in all prose, except inside code, commands, URLs, quoted upstream text, or exact API names.

Keep eddacraft-owned brand and product names lowercase in prose, including `eddacraft`, `anvil`, and `nxrust`.

Prefer extracting durable ideas over adopting implementations.

Assume every dependency introduces long-term cost until proven otherwise.

A successful assessment answers:

"Should this consume anvil roadmap space?"

## Philosophy

Do not look for competitors to copy.

Do not look for code to copy.

Look for product primitives that make anvil:

- More trustworthy
- More auditable
- More explainable
- More governable
- More developer-friendly
- Easier to adopt
- More strategically differentiated

The reason this skill is biased so hard towards primitives over implementations is that anvil's durable advantage is its governance and evidence model, not any one piece of code. Code ages, accrues maintenance cost, and ties anvil to someone else's design decisions; a well-understood primitive can be expressed in anvil's own terms, on anvil's own roadmap, and outlives the project it was borrowed from. So when a project looks valuable, the question is what idea it teaches, not what files it ships.

The most valuable outcome is often:

"Interesting idea. No implementation worth reusing."

## Use This Skill When

- Morgan proposes a repository or product
- A GitHub project appears strategically interesting for anvil
- A dependency adoption decision is being considered for anvil
- A clean-room reimplementation decision is required
- A new capability is being considered for anvil
- A primitive needs evaluation
- A roadmap opportunity needs validation
- The user asks whether anvil should care about a candidate project or idea

Do not use this skill for generic repository reviews, implementation help, dependency audits, or code review unless strategic relevance to anvil is explicit or strongly implied.

## Inputs

You may receive any of:

- A repository URL
- A product URL
- A paper
- A specification
- A product description
- A Morgan recommendation
- A Morgan hypothesis

Example:

> Repository: https://github.com/example/project
>
> Morgan Hypothesis: This repository appears to contain a useful capability discovery model that may strengthen anvil's ability to explain available agent capabilities and policy boundaries.

Treat Morgan's hypothesis as a starting point. Attempt to validate or disprove it. Morgan is a lead, not a verdict.

## Assessment Workflow

Follow this workflow in order. Each step exists to stop a specific failure mode, noted alongside it.

### Step 1 - Understand the Candidate

Determine what it is, what problem it solves, who it serves, and why it exists.

Do not evaluate implementation details before understanding the purpose. Failure mode: judging elegant code that solves a problem anvil does not have.

### Step 2 - Identify the Primitive

Identify the single most valuable primitive contained within the project. Examples: evidence receipt, capability discovery, approval workflow, authority delegation, decision provenance, policy explanation, source quality assessment, trust scoring, agent evaluation, risk classification, governance workflow.

If no meaningful primitive can be identified, score poorly. Failure mode: mistaking a feature list for a reusable idea.

### Step 3 - Validate the Hypothesis

If Morgan supplied a hypothesis, confirm it, partially confirm it, or reject it, and actively search for a better explanation than the one offered. Do not assume Morgan is correct. Failure mode: anchoring on the first framing.

### Step 4 - Apply the Customer Surface Test

Ask: "What customer-visible capability becomes stronger because of this?" Examples: better evidence, governance, auditability, policy enforcement, explainability, or trust.

If customer impact is weak, downgrade the recommendation. Do not confuse engineering elegance with customer value. Failure mode: scoring the demo, not the buyer outcome.

### Step 5 - Assess Strategic Relevance

Determine whether the candidate strengthens governance, trust, evidence, provenance, policy, approvals, auditability, or operational confidence. Projects that do not materially improve these areas should score poorly. Failure mode: roadmap drift into adjacent-but-off-mission territory.

### Step 6 - Assess Licensing

Always evaluate licensing where applicable: licence type, commercial compatibility, attribution requirements, copyleft obligations, dependency suitability, vendoring suitability, and clean-room suitability.

For non-code candidates such as papers, specifications, or product descriptions, record the relevant publication terms, specification licence, or "Not applicable". If licensing or reuse terms are unclear, treat that as a risk.

Failure mode: discovering a copyleft obligation or reuse restriction after the idea is already in the tree.

### Step 7 - Determine Acquisition Strategy

Select the best way to capture the value. Prefer concepts over implementations.

Direct use, vendoring, or adaptation require explicit justification: licensing clarity, maintenance confidence, strategic leverage, and a reason clean-room reimplementation is insufficient.

### Step 8 - Determine Roadmap Disposition

Every assessment ends with a clear disposition and recommended next step. Do not leave the outcome ambiguous.

## Evaluation Criteria

Score each criterion from 0-10. The ten criteria sum to a total out of 100.

Use this calibration unless a criterion has a more specific reason:

- 0-2: No meaningful evidence or actively misaligned
- 3-4: Weak, indirect, or speculative fit
- 5-6: Plausible fit with unresolved concerns
- 7-8: Strong fit with clear anvil value
- 9-10: Direct, compelling, differentiated anvil value

| Criterion                   | What it asks                                            |
| --------------------------- | ------------------------------------------------------- |
| Direct anvil Fit            | Does this strengthen anvil's core mission?              |
| Borrowable Primitive        | Can a concrete reusable primitive be identified?        |
| Developer-Native Usefulness | Does it fit naturally into developer workflows?         |
| Evidence Before Enforcement | Does it demonstrate value before becoming a gate?       |
| Deterministic Governance    | Can it become deterministic and auditable?              |
| Audit and Export Value      | Does it improve what anvil can prove later?             |
| Narrow Beta Wedge           | Can it be introduced incrementally?                     |
| Strategic Differentiation   | Does it help anvil remain distinct?                     |
| Clean-Room Feasibility      | Can the capability be recreated independently?          |
| Buyer Language Strength     | Does it improve language buyers immediately understand? |

## Acquisition Strategies

This axis describes **how** anvil would capture the value if it chose to. It captures the IP and dependency posture. It is orthogonal to the roadmap disposition: a single candidate has both an acquisition strategy and a disposition.

Choose exactly one primary strategy:

- **Use Directly** - use as a dependency, integration, or external component.
- **Vendor** - bring the implementation into anvil.
- **Adapt** - reuse implementation concepts while modifying heavily.
- **Clean-Room Reimplementation** - recreate the capability independently.
- **Inspiration Only** - useful idea, but the implementation is not worth taking.
- **Reject** - not strategically valuable.

If there is uncertainty, still choose one primary strategy and call out the blocker or secondary consideration in the reasoning.

## Reject Conditions

Strongly consider rejection if any hold:

- No meaningful primitive exists
- Customer value is weak
- Strategic relevance is weak
- The capability is too broad
- Adoption would bloat anvil
- Dependency cost exceeds value
- The value is primarily engineering elegance

A technically impressive project is not automatically a good opportunity.

## Output Format

Produce the assessment using this exact structure.

### anvil opportunity assessment

#### Executive Summary

Brief summary and overall recommendation.

#### Roadmap Disposition

Choose exactly one, explain the reasoning, and identify the recommended next step. This is a recommendation only; do not perform the follow-up work unless explicitly requested.

- **Reject**
- **Track**
- **Product Note**
- **Specification**
- **APS Plan**
- **Prototype**
- **Dependency Evaluation**

#### Recommended Next Step

State the single recommended follow-up artefact from Escalation Guidance. Do not produce the artefact unless requested.

#### Candidate Primitive

- **Name:**
- **Description:**
- **Why it matters to anvil:**

#### Morgan Hypothesis Assessment

- **Status:** Confirmed / Partially Confirmed / Rejected / Not Provided
- **Explanation:**

#### Better Primitive Identified

If applicable, describe a more valuable primitive discovered during analysis. Omit if none.

#### Customer Surface Test

What customer-facing capability becomes stronger, and why would a customer care? If customer impact is weak, state so explicitly.

#### Criteria Scorecard

| Criterion                   | Score      |
| --------------------------- | ---------- |
| Direct anvil Fit            | X          |
| Borrowable Primitive        | X          |
| Developer-Native Usefulness | X          |
| Evidence Before Enforcement | X          |
| Deterministic Governance    | X          |
| Audit and Export Value      | X          |
| Narrow Beta Wedge           | X          |
| Strategic Differentiation   | X          |
| Clean-Room Feasibility      | X          |
| Buyer Language Strength     | X          |
| **Overall**                 | **XX/100** |

#### Key Ideas Worth Exploring

List the most valuable ideas. For each, include description, value to anvil, and recommended action.

#### Patterns and Processes Worth Replicating

Focus on mechanisms rather than features, such as workflow design, governance model, evidence lifecycle, approval lifecycle, state management, operational model, or agent interaction pattern.

#### Licensing Assessment

- **Licence:**
- **Risk Level:**
- **Dependency Suitability:**
- **Vendoring Suitability:**
- **Clean-Room Preference:**
- **Notes:**

For non-code candidates, use "Not applicable" where appropriate and record publication, specification, or reuse terms in Notes.

#### Acquisition Strategy

- **Selected Strategy:** exactly one from Acquisition Strategies
- **Reasoning:**

#### anvil Integration Surface

Identify where the capability would appear, such as policy check, evidence generator, receipt format, approval workflow, export format, CLI command, daemon feature, RMCP capability, or developer-experience feature.

#### Risks and Concerns

Technical, strategic, product, licensing, and operational risks.

#### Final Verdict

A single sentence in this form: `If I were making the decision today, I would ______ because ______.`

## Escalation Guidance

The Roadmap Disposition determines the recommended follow-up artefact. Do not create the follow-up artefact unless the user explicitly asks. Include the recommended next step in the assessment.

| Disposition           | Recommended follow-up                                 |
| --------------------- | ----------------------------------------------------- |
| Reject                | None. Record the rejection and move on.               |
| Track                 | Create a primitive catalogue entry.                   |
| Product Note          | Draft a concise product thesis.                       |
| Specification         | Draft a specification.                                |
| APS Plan              | Generate APS planning artefacts.                      |
| Prototype             | Create an implementation experiment proposal.         |
| Dependency Evaluation | Perform a deeper dependency and licensing assessment. |
