# Constitutional Engineering

| Type  | Authority | Owner  | Status | Freshness                                        |
| ----- | --------- | ------ | ------ | ------------------------------------------------ |
| Guide | Advisory  | VISION | Live   | Metadata backfilled 2026-05-27 during DOCGOV-011 |

| Upstream          | Downstream                                     |
| ----------------- | ---------------------------------------------- |
| Anvil vision docs | Architecture specs and kernel policy rationale |

**This is a philosophical document exploring the conceptual foundations of
Anvil's approach. It is not a scope document or implementation plan.**

## Definition

**Constitutional engineering is the design and enforcement of non-negotiable
structural invariants that govern how a software system is allowed to evolve.**

It is not about features. It is not about code style. It is about _what kinds of
change are legitimate_.

Just as a political constitution:

- Defines separation of powers
- Limits authority expansion
- Establishes rights and constraints
- Controls how change can occur

A software constitution defines:

- Allowed architectural boundaries
- Permitted trust transitions
- Data classification constraints
- Public interface growth rules
- Dependency exposure rules
- Privilege escalation constraints

And crucially:

It defines **how those rules are enforced.**

---

## Why This Is Different From “Governance”

Traditional governance is advisory and reactive.

Constitutional engineering is:

- Deterministic
- Structural
- Enforced
- Evolution-aware

Most tools answer:

> “Is this code valid?”

Constitutional systems answer:

> “Is this code evolution legitimate?”

That’s a higher order question.

---

## The Three Layers of Constitutional Engineering

### 1. Structural Law

Hard invariants:

- No cross-layer imports
- No external network calls in high-trust modules
- No privilege expansion without declaration
- Public API changes require explicit versioning intent

These are deterministic. Non-negotiable.

---

### 2. Evolution Law

Rules about change itself:

- Public surface growth rate limits
- Dependency sprawl constraints
- Trust surface expansion constraints
- Async execution introduction review gates

This governs drift.

---

### 3. Procedural Law

Rules about _how change is introduced_:

- Must map to an active plan
- Must declare boundary shifts
- Must declare new data classifications
- Must declare new external dependencies

This is where APS becomes constitutional.

---

## Anticipatory Tooling

Now the dangerous part.

Most tools are reactive. They evaluate _after_ something exists.

Anticipatory tooling attempts to model trajectory.

Instead of:

> “You violated a boundary.”

It says:

> “You are trending toward structural instability.”

That is a completely different capability.

---

## What Is Anticipatory Tooling?

Anticipatory tooling is software that:

- Maintains a model of expected architecture
- Tracks rate and direction of structural change
- Detects emerging patterns before violation
- Flags entropy accumulation
- Warns about privilege concentration trends

It is not predicting the future mystically.

It is detecting _trajectory vectors_.

---

## The Architectural Immune System Model

Think biologically.

Reactive system:

- Detect infection.
- Respond after damage begins.

Anticipatory immune system:

- Detect abnormal cell growth patterns.
- Intervene before malignancy forms.

Constitutional engineering plus anticipatory tooling equals:

A structural immune system for software.

---

## Concrete Anticipatory Capabilities

These are implementable.

## 1. Boundary Stress Index

Measure:

- Cross-layer import frequency
- Edge density between layers
- Unauthorized dependency attempts (even if later fixed)

Track trend line.

If boundary stress accelerates → warning.

---

## 2. Privilege Drift Index

Track:

- Functions touching high-trust resources
- Privileged modules gaining new callers
- Authentication bypass pattern attempts

If privilege surface expands unusually → warning.

---

## 3. API Surface Growth Monitoring

Track:

- Number of public exports over time
- Expansion velocity
- Inconsistent versioning intent

Warn if public interface grows without plan alignment.

---

## 4. External Surface Creep

Track:

- New outbound network calls
- New external dependencies
- New inbound exposure points

Alert before the external surface becomes chaotic.

---

## 5. Entropy Gradient Detection

Measure:

- Dependency graph complexity (edges per node)
- Cycles introduced
- Increased coupling density

Entropy is measurable.

Entropy trend is anticipatory signal.

---

## Why This Matters Strategically

Most engineering teams do not fail because they break a rule.

They fail because:

- They drift slowly.
- They accrete complexity invisibly.
- They expand trust surface accidentally.
- They normalise boundary erosion.

By the time a violation is visible, the architecture is already sick.

Anticipatory tooling says:

> “Your architecture is trending toward instability.”

That changes behaviour earlier.

---

## Constitutional Engineering + Anticipatory Tooling

Put together:

Constitutional engineering defines the invariants.

Anticipatory tooling monitors the trajectory toward breaking them.

One is law. The other is early warning radar.

Together, they form:

A deterministic evolutionary control system.

---

## Where Anvil Fits

Anvil is not a scanner.

If built correctly, it becomes:

- The constitutional runtime of the repository.
- The invariant enforcement kernel.
- The structural trend analysis engine.
- The evolution legitimacy gate.

That’s not DevEx tooling.

That’s civilisation for codebases.

---

## The Philosophical Angle (But Still Practical)

Most software systems decay because:

Change is cheap. Structure is invisible. Entropy is silent.

Constitutional engineering makes structure explicit.

Anticipatory tooling makes entropy visible.

When both exist, you stop asking:

> “Does it compile?”

And start asking:

> “Is this evolution legitimate?”

That’s a different engineering culture entirely.

And it is buildable.

This is where Anvil stops being a tool and becomes infrastructure.
