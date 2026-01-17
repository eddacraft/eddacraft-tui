---
id: what-is-eddacraft
title: What is EddaCraft?
description: EddaCraft is the umbrella organisation building tools for governed AI-assisted development.
sidebar_position: 1
---

# What is EddaCraft?

**EddaCraft** is the forge for governed AI-assisted work. We build tools that make AI-generated code changes safe for production—without sacrificing velocity.

## The Problem

AI coding tools produce code that compiles and passes tests. But they drift from your intended architecture. They introduce anti-patterns. They make changes that look correct but erode your codebase over time.

Code review catches some of this. But by then, the damage is done—and the cognitive load on reviewers is unsustainable.

## Our Approach

We believe in **restraint over velocity**. Our tools enforce quality gates *before* code reaches review, giving you confidence that AI-generated changes align with your standards.

```
AI Agent → Plan → Gate → Execute → Evidence
```

Every change is:
- **Planned** — defined intent before execution
- **Gated** — validated against quality and policy checks
- **Evidenced** — immutable audit trail with provenance

## The Products

EddaCraft develops three interconnected tools:

### Anvil

The flagship product. Anvil validates AI-generated code changes through deterministic gates at save-time, catching architecture drift and anti-patterns before they reach review.

**Best for:** Teams shipping with AI assistance who need production-grade quality gates.

[Get started with Anvil →](/docs/anvil/overview)

### APS (Anvil Plan Specification)

An open-source specification for deterministic, hash-stable development plans. APS defines *what* should be built, enabling reproducible validation.

**Best for:** Teams wanting a standard format for AI-agent task definitions.

[Explore the APS spec →](/docs/aps/overview)

### Kindling

An open-source tool for capturing structured observations from development sessions. Kindling stores context that matters—without the noise.

**Best for:** Developers wanting session memory that persists and transfers.

[Learn about Kindling →](/docs/kindling/overview)

## Design Principles

### Provenance Matters

Every artefact traces back to its origin. Who created it, when, from what inputs, with what validation.

### Determinism Enables Trust

Given the same inputs, you get the same outputs. Hash-stable plans mean reproducible validation.

### Progressive Disclosure

Simple things should be simple. Complex things should be possible. Our tools scale from solo dev to enterprise.

### Open Where It Counts

The specifications and memory layer are open source. The governance tooling builds on open foundations.

---

**Ready to dive in?** [Choose your path →](/docs/start-here/choose-your-path)
