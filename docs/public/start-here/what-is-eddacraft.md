---
id: what-is-eddacraft
title: What is eddacraft?
description:
  eddacraft is the umbrella organisation building tools for governed AI-assisted
  development.
sidebar_position: 1
---

# What is eddacraft?

**eddacraft** is the forge for governed AI-assisted work. We build tools that
make AI-generated code changes safe for production—without sacrificing velocity.

## The Problem

AI coding tools produce code that compiles and passes tests. But they drift from
your intended architecture. They introduce anti-patterns. They make changes that
look correct but erode your codebase over time.

Code review catches some of this. But by then, the damage is done—and the
cognitive load on reviewers is unsustainable.

## Our Approach

We believe in **restraint over velocity**. Our tools enforce quality gates
_before_ code reaches review, giving you confidence that AI-generated changes
align with your standards.

```
AI Agent → Plan → Gate → Execute → Evidence
```

Every change is:

- **Planned** — defined intent before execution
- **Gated** — validated against quality and policy checks
- **Evidenced** — immutable audit trail with provenance

## The Products

eddacraft develops tools across three capability areas:

- Deterministic planning
- Save-time governance
- Reusable development memory

### anvil

The flagship product. anvil validates AI-generated code changes through
deterministic gates as they are made, catching architecture drift and
anti-patterns before they reach review.

**Best for:** Teams shipping with AI assistance who need production-grade
quality gates.

[Get started with anvil →](/anvil/overview)

### APS (anvil plan specification)

An open-source specification for deterministic, hash-stable development plans.
APS defines _what_ should be built, enabling reproducible validation.

**Best for:** Teams wanting a standard format for AI-agent task definitions.

[Explore the APS spec →](/aps/overview)

### kindling

An open-source tool for capturing structured observations from development
sessions. kindling stores context that matters—without the noise.

**Best for:** Developers wanting session memory that persists and transfers.

[Learn about kindling →](/kindling/overview)

### Development Memory System (kindling + ember + edda)

Our memory system turns raw observations into trusted, reusable team knowledge.
It is designed to reduce repeat mistakes, preserve decision context, and improve
onboarding for both engineers and agents.

**Best for:** Teams that want capability-first memory workflows: capture,
review, and canonical preservation.

[Explore the memory system →](/edda-stack/overview)

## Design Principles

### Provenance Matters

Every artefact traces back to its origin. Who created it, when, from what
inputs, with what validation.

### Determinism Enables Trust

Given the same inputs, you get the same outputs. Hash-stable plans mean
reproducible validation.

### Progressive Disclosure

Simple things should be simple. Complex things should be possible. Our tools
scale from solo dev to enterprise.

### Open Where It Counts

The specifications and memory layer are open source. The governance tooling
builds on open foundations.

---

**Ready to dive in?** [Choose your path →](/start-here/choose-your-path)
