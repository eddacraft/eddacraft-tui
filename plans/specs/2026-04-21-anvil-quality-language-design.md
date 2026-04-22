# Anvil Quality Language Design

## Purpose

This document is the `CLAR-003` output. It defines the canonical language and
user mental model for Anvil's quality system so current and future surfaces can
teach the same product, not parallel dialects.

It is grounded in the discovery inventory in
`plans/specs/2026-04-21-check-language-inventory.md`, including both shipped
surfaces and forward-looking APS plans/specs.

## Problem Statement

Anvil currently exposes multiple overlapping noun systems for adjacent ideas:

- `check` in config, doctor, gate, tutorial, and policy testing
- `gate` as quality run, workflow barrier, docs access control, and future
  enforcement language
- `scan` as onboarding discovery action, engine implementation detail, and
  shorthand for several specific detectors
- `warning`, `issue`, `finding`, and `violation` as partially overlapping
  outcome terms
- `architecture`, `boundary`, `graph`, and `policy` without a stable hierarchy

The result is that new users are shown commands and labels, but are not taught
a coherent model of what Anvil actually does.

## Design Goal

Give every core surface the same answer to five questions:

1. What does Anvil inspect?
2. What is the smallest unit of evaluation?
3. What is a gate?
4. What kind of outcomes can a user see?
5. How do architecture, policy, graph, watch, audit, and tutorial relate to
   that model?

## Canonical Mental Model

### Short version

Anvil analyses your codebase and configuration to understand structure,
dependencies, boundaries, and policy context. It runs **checks** against that
understanding. A **gate** is the workflow decision produced from one or more
checks. The results appear as **findings**, with severity and guidance. Some
surfaces are for setup (`doctor`), some for broad exploration (`audit`), some
for continuous feedback (`watch`), and some for guided learning (`tutorial`).

### User-facing hierarchy

1. **Graph / structure**
   Anvil builds a structural understanding of the project: files, imports,
   boundaries, layers, dependencies, and related context.
2. **Checks**
   A check evaluates one concern against the project or its graph.
3. **Findings**
   A check can emit findings, each with severity, message, and remediation.
4. **Gate**
   A gate is the workflow decision over a set of checks: pass, warn, or fail.
5. **Modes and surfaces**
   Commands such as `watch`, `doctor`, `audit`, and `tutorial` are ways of
   interacting with checks, findings, and gates for different purposes.

## Canonical Terms

### `check`

**Definition:** The smallest user-facing unit of evaluation in Anvil.

Use `check` for:

- configured project checks
- gate sub-results
- doctor diagnostics when they are truly individual evaluations
- future dashboard entries that represent one evaluative rule or unit

Do not use `check` for:

- the overall run across many checks
- architecture config files themselves
- policy packs as a whole unless referring to a specific evaluative unit

### `gate`

**Definition:** A workflow decision over one or more checks.

This intentionally preserves the blocking/workflow connotation you called out:
a gate is not just a bag of results, it is the thing a user goes through to
advance.

Use `gate` for:

- `anvil gate`
- gate results, gate profiles, gate decisions
- dashboard or CI surfaces that answer “can this advance?”

Do not use `gate` for:

- generic access control unless the workflow-barrier meaning is intended and
  obvious in context
- every policy or validation surface by default
- low-level implementation registries

Implication: other planned docs that use `gate` as generic “control plane” or
“preflight” wording should be reviewed for ambiguity.

### `finding`

**Definition:** The canonical generic noun for a problem, observation, or alert
emitted by a check.

Why `finding`:

- broader than `violation`
- less overloaded than `warning`
- more consistent across security, architecture, policy, and quality domains

Use `finding` for the generic cross-surface noun.

Subtypes can remain when needed:

- `violation` for a rule or boundary breach
- `warning` for severity or non-blocking state
- `issue` only where a dedicated audit UX deliberately groups mixed concerns

### `scan`

**Definition:** An analysis action that discovers data or findings.

Use `scan` for:

- first-run discovery scan
- secret scanning / antipattern scanning as actions
- implementation-level scanner components

Do not use `scan` as the primary top-level taxonomy for the product.

`scan` should describe how Anvil gathers evidence, not replace `check` or
`gate` as the user model.

### `graph`

**Definition:** Anvil's structural understanding of the codebase.

Use `graph` in explanatory material because it is a real differentiator.
However, do not force it into first-run UX before users understand checks and
gates. Teach it as the reason Anvil can perform architectural checks well.

### `boundary`

**Definition:** A declared structural constraint about what parts of the system
may depend on one another.

This should remain a canonical term. It is clearer than `architecture` when the
actual topic is dependency constraints.

### `policy`

**Definition:** A declarative rule set evaluated against Anvil input.

Policies are one family of checks. They should not absorb architecture checks by
default unless the implementation is truly unified and the user benefit is clear.

## Preferred Product Framing

The product should be taught as:

- Anvil understands the structure of your codebase
- It runs checks against that structure and your project configuration
- Those checks produce findings
- Gates summarise whether work is safe to advance

Not as:

- a loose pile of scans, audits, warnings, and policy engines
- a list of commands the user must memorise before understanding the model

## Command and Surface Framing

### `anvil gate`

Primary workflow decision surface.

- Runs a selected set of checks
- Produces findings and an overall gate decision
- Should be described as the command that decides whether work passes quality
  gates

### `anvil doctor`

Setup and environment health surface.

- Uses checks, but they are setup checks
- Should teach readiness, not project quality semantics

### `anvil audit`

Broad exploratory review surface.

- Can keep `issue` for its own UX if useful
- But docs and tutorials should explain that audit surfaces findings across the
  repo rather than representing the canonical gate model

### `anvil watch`

Continuous mode.

- Re-runs checks and gate decisions as files change
- Should be presented as a mode over checks/gates, not as a separate quality
  system

### `anvil architecture`

Configuration and structure-definition surface.

- `architecture validate` should mean config validity and model integrity
- boundary-enforcement results should ideally be described as boundary checks or
  architecture checks within gate, but the distinction must be explicit

### `anvil policy`

Policy authoring, inspection, validation, and testing surface.

- Policy remains a specialised subsystem
- Policy checks can participate in gates, but policy is not the only check type

### `tutorial` and onboarding

These should teach the model in this order:

1. Anvil analyses your project
2. It runs checks
3. Checks emit findings
4. Gates summarise whether you can advance
5. Watch mode repeats that loop continuously

Commands should come after the concepts, not before them.

## Term Mapping

| Current Term | Canonical Handling | Notes |
| --- | --- | --- |
| `checks` | keep | Canonical smallest evaluation unit |
| `gate` | keep, but narrow | Aggregate workflow decision, not generic synonym for control |
| `scan` | keep as action term | Discovery/analysis verb, not top-level taxonomy |
| `warning` | keep as severity/state term | Not the generic noun for all results |
| `violation` | keep as subtype | Specific breach result |
| `finding` | promote | Generic result noun across product |
| `issue` | contain to audit UX | Avoid as cross-product canonical noun |
| `architecture` | narrow | Use for model/config/domain, not every boundary result |
| `boundary` | keep and strengthen | Clear user-facing structural concept |
| `graph` | keep in explanation | Important differentiator, but second-step teaching term |
| `secret-detection` | likely canonical check name candidate | Better user term than bare `secret` |
| `import-boundaries` | likely canonical check name candidate | Better user term than overloading `architecture` |
| `antipattern-scan` | review | Needs a clear place in the model or removal from defaults |

## Guidance For Forward-Looking Plans

All active and draft APS modules should align with the following rules when
describing new commands, screens, routes, and surfaces:

1. Use `check` for the smallest evaluative unit.
2. Use `gate` only when describing workflow advancement or blocking judgement.
3. Use `finding` as the generic result noun unless a subtype is required.
4. Use `scan` for evidence-gathering actions, not as the product's primary noun.
5. Use `boundary` when talking about dependency constraints.
6. Introduce `graph` deliberately as the structural reason behind checks, not as
   unexplained jargon in first-run UX.

### Plans that likely need wording review

- dashboard modules that mention gate detail, check tree, and dependency graph
- policy governance modules that use `gate` in governance/control meanings
- intercept loop modules that may over-introduce blocking/enforcement language
- release and feature-flag specs that use `gate` for unrelated workflow controls

## Onboarding Sequence

Recommended teaching order for welcome/tutorial/docs:

1. **What Anvil sees:** structure, dependencies, boundaries
2. **What Anvil runs:** checks
3. **What Anvil reports:** findings with severity and guidance
4. **What Anvil decides:** gate outcome for workflow advancement
5. **How you use it daily:** watch mode, policy tools, architecture tools,
   audit, dashboard

## Immediate Implications

### Highest-priority wording changes to plan for

- align config/onboarding check names with gate-runner names
- decide whether `import-boundaries` replaces gate `architecture`, or whether a
  different split is explicitly documented
- introduce `finding` as the shared generic result noun in docs/onboarding
- audit tutorial and welcome copy so concepts are explained before commands

### Things not to do yet

- do not rename internal crates or types solely to match this language
- do not collapse audit, doctor, and policy into one UX model if their
  operational roles remain distinct
- do not make `graph` mandatory jargon in every surface

## Follow-On Work

This design should feed `CLAR-004` and `CLAR-005`.

Expected implementation slices:

- check naming and registry unification
- onboarding and tutorial copy rewrite
- docs terminology cleanup
- active APS wording reconciliation for future modules
