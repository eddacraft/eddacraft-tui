# Anvil Vision

| Type  | Authority     | Owner  | Status | Freshness                                                             |
| ----- | ------------- | ------ | ------ | --------------------------------------------------------------------- |
| Guide | Authoritative | VISION | Live   | "Posture today" note added 2026-07-02 (aspiration vs shipped default) |

| Upstream             | Downstream                             |
| -------------------- | -------------------------------------- |
| Product vision input | Scope guard, architecture, public docs |

Anvil exists to ensure that AI and humans cannot produce unsafe software.

## The Shift

Software creation has changed.

AI can now generate:

- Code
- Infrastructure
- Architecture decisions
- Operational changes

The constraint is no longer ability to build. The constraint is control over
what is being built.

## The Problem

Without control, AI accelerates:

- Anti-patterns (architecture drift, poor design decisions)
- Security risks (misconfigurations, vulnerabilities)
- Policy violations (compliance, governance gaps)
- Unintended consequences at scale

Traditional approaches rely on:

- Code review
- CI/CD checks
- Post-deployment monitoring

These happen too late.

## The Anvil Approach

Anvil is a real-time control layer for software creation.

It operates at the moment change is generated — not after.

Anvil:

- Intercepts changes as they are created
- Validates them against deterministic rules and policies
- Prevents unsafe or non-compliant outcomes from executing
- Enforces architectural, security, and organisational constraints
- Captures provenance for every decision and action

### Posture today

This is the north star. The **shipped default is advisory**: Anvil surfaces
findings as warnings and exits 0 (ADR-002, "warnings over blocks"), warning only
on _new_ violations against a baseline. Hard prevention — failing a change
before it lands — is real but **opt-in**: `--fail-on-warnings` on the
check/policy path, and the intercept daemon's enforcement mode on the save-time
path. Read "prevents ... from executing" below as the capability Anvil is built
to provide and operators can switch on, not as the zero-config behaviour every
user gets on day one.

## Core Principles

### 1. Prevention over detection

Problems should not be found later — they should never occur.

### 2. Deterministic over probabilistic

AI is probabilistic. Control must be deterministic.

### 3. Real-time enforcement

Governance must exist at the point of creation, not after commit.

### 4. Behavioural control, not just code quality

The system governs decisions, not just outputs.

### 5. Provenance is mandatory

Every action must be traceable, explainable, and attributable.

## The Outcome

With Anvil:

- Unsafe changes do not execute
- Anti-patterns do not enter the system
- Policies are enforced automatically
- AI operates within controlled boundaries
- Teams move faster without losing control

## North Star

Move governance from after-the-fact review to deterministic, pre-execution
control.
