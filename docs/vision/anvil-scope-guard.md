# Anvil Scope Guard

| Type  | Authority     | Owner  | Status | Freshness                                              |
| ----- | ------------- | ------ | ------ | ------------------------------------------------------ |
| Guide | Authoritative | VISION | Live   | Decision-framework reconciled to ADR-002 on 2026-07-02 |

| Upstream     | Downstream                                     |
| ------------ | ---------------------------------------------- |
| Anvil vision | Architecture decisions, APS scoping, PR review |

## Purpose

This document defines the strict boundaries of what Anvil is and is not.

Its purpose is to ensure that all contributions — human or agent — reinforce
Anvil's role as a real-time control layer for safe software creation, and
prevent scope drift over time.

## North Star

Anvil ensures that AI and humans cannot produce unsafe software.

## Core Definition

Anvil is a deterministic control layer that operates at the point of change
creation.

It exists to:

- Intercept changes as they are generated
- Validate them against policies and rules
- Prevent unsafe, non-compliant, or anti-patterned outcomes
- Enforce architectural, security, and organisational constraints
- Capture provenance for every decision and action

## In-Scope Capabilities

The following are core to Anvil:

### 1. Prevention & Enforcement

- Blocking unsafe changes before execution
- Enforcing policy as code
- Real-time validation of generated outputs

### 2. Anti-Pattern Detection

- Identifying architectural drift
- Preventing known bad practices
- Enforcing system design constraints
- Security anti-patterns are **in scope only as syntactic-construction smells**
  — weak crypto primitives, DOM-XSS sinks, JWT misuse (the
  `insecure-construction` category, ADR-087). Taint-class findings
  (reflected/stored XSS, output encoding) and absence-class findings (missing
  rate limiting, MFA, CSP, server-side validation) are **out of model**: Anvil
  is a deterministic single-file regex/AST scanner, not a taint-based SAST, and
  crossing that boundary requires a new ADR.

### 3. Deterministic Validation

- Rule-based decision systems
- Reproducible validation outcomes
- Elimination of ambiguity in enforcement

### 4. Pre-Execution Control

- Operating before commit, deploy, or runtime
- Intercepting agent and human actions at creation time

### 5. Provenance & Traceability

- Recording why decisions were made
- Linking actions to policies and rules
- Ensuring full auditability

## Out-of-Scope Capabilities

The following are explicitly not Anvil:

### 1. General-Purpose Agent Systems

- No agent orchestration platform
- No multi-agent coordination frameworks
- No autonomous planning engines

### 2. Developer Productivity Tools

- No generic code generation features
- No IDE-like enhancements unrelated to safety
- No workflow acceleration for its own sake

### 3. CI/CD Replacement

- Not a pipeline runner
- Not a deployment engine
- Not a build system

### 4. Planning or Ideation Systems

- Not a product planning tool
- Not a requirements system
- Not a task management platform

### 5. Observability Platforms

- Not a monitoring system
- Not a logging platform
- Not a metrics dashboard

(Exception: observability that directly supports enforcement or provenance is
allowed)

## Borderline Cases (Decision Framework)

When evaluating a feature, apply:

1. **Does it increase prevention capability?** If not → Reject

2. **Does it operate before or at execution time?** If only after → Reject

3. **Does it strengthen deterministic control?** If probabilistic → Reject

4. **Does it serve enforcement, or only inform for its own sake?** Anvil's
   shipped default is _deterministic advisory_ — warn on new violations, exit 0,
   with enforcement opt-in (ADR-002). So a deterministic finding that an
   operator can promote to a hard failure (`--fail-on-warnings`, intercept
   enforcement) is **in scope**; what is rejected is _probabilistic_ or
   _decorative_ advice that never becomes enforceable — suggestions, nudges,
   quality scores with no path to a deterministic gate.

## Allowed vs Not Allowed Examples

| Scenario                                        | Decision    | Reason                      |
| ----------------------------------------------- | ----------- | --------------------------- |
| Blocking insecure Terraform config before apply | Allowed     | Direct prevention           |
| Suggesting better architecture patterns         | Not allowed | Advisory, not enforcement   |
| Enforcing repo structure via policy             | Allowed     | Deterministic control       |
| Generating boilerplate code                     | Not allowed | Productivity, not safety    |
| Capturing decision provenance                   | Allowed     | Core requirement            |
| Dashboard showing violations                    | Conditional | Only if tied to enforcement |

## Guiding Principle

Anvil is not here to help build software faster.

It is here to ensure software is built correctly and safely.

## Final Rule

If a feature does not increase Anvil's ability to prevent unsafe outcomes, it
does not belong in Anvil.
