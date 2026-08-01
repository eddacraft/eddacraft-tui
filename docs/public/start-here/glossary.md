---
id: glossary
title: Glossary
description: Definitions of key terms used across eddacraft documentation.
sidebar_position: 3
---

# Glossary

Key terms used throughout eddacraft documentation. We use forge metaphors
sparingly—each term has a plain definition alongside.

---

## Core Concepts

### anvil

The eddacraft tool for validating AI-generated code changes. Named for the
blacksmith's anvil—the stable surface where raw material is shaped into
something reliable.

**Plain definition:** A CLI and service that runs quality gates on code changes
before they reach review.

### APS (anvil plan specification)

A deterministic, hash-stable format for defining development plans. The
specification that anvil validates against.

**Plain definition:** A structured document format (Markdown with frontmatter)
that describes what should be built, enabling reproducible validation.

### kindling

A tool for capturing structured observations from development sessions. Named
for the small material that catches fire first—observations that spark larger
understanding.

**Plain definition:** A memory capture system that stores context from coding
sessions in a structured, searchable format.

---

## Plan Structure

### Index

The root document of an APS plan. Lists all modules and provides project-level
metadata.

**Plain definition:** The `index.aps.md` file that ties together all parts of a
plan.

### Module

A cohesive unit of functionality within a plan. Contains related tasks.

**Plain definition:** A group of related tasks, typically mapping to a feature
or component.

### Task

A unit of authorised work. Defines an outcome and how to validate it.

**Plain definition:** A specific piece of work with clear success criteria and a
validation command.

### Step

A checkpoint within task execution. Observable state only—no implementation
details.

**Plain definition:** A milestone that describes _what_ should be true, not
_how_ to achieve it.

---

## Validation

### Gate

A quality check that code must pass before proceeding. Gates are deterministic
and composable.

**Plain definition:** A validation rule (lint, test, coverage, policy) that
blocks or allows changes.

### Check

A single validation rule within a gate. Examples: ESLint check, coverage check,
secret detection.

**Plain definition:** One specific test applied during gate validation.

### Evidence

The immutable record of a validation run. Includes inputs, outputs, and
provenance.

**Plain definition:** A timestamped, signed record proving what was validated
and the result.

---

## Memory (Kindling)

### Capsule

A container for observations from a bounded context (typically a session or
project).

**Plain definition:** A named collection of observations, stored together for
retrieval.

### Observation

A single piece of captured context. Has a kind, content, and provenance.

**Plain definition:** A structured note with metadata about where it came from.

### Provenance

The origin and lineage of data. Who created it, when, from what source.

**Plain definition:** Metadata that traces data back to its source.

---

## Architecture

### Edda Stack

The complete eddacraft architecture: Kindling (capture) → Ember (candidate) →
Edda (curated).

**Plain definition:** The layered system for capturing, promoting, and curating
development memory.

### Ember

_(Planned)_ Candidate memory awaiting promotion to curated status.

**Plain definition:** Observations that have been flagged as potentially
valuable but not yet verified.

### Edda

_(Planned)_ Curated, verified knowledge extracted from observations.

**Plain definition:** The final layer of trusted, reusable knowledge.

---

## Workflow

### Session

A bounded period of development work, typically with a specific goal.

**Plain definition:** A time-boxed unit of work, from start to commit/abandon.

### Run

A single execution of anvil validation against a codebase.

**Plain definition:** One invocation of `anvil check` or equivalent.

### Artefact

Any file or data produced during execution that may be referenced later.

**Plain definition:** Build outputs, reports, logs, or other generated files.

---

## Integration

### Adapter

A component that connects external tools to eddacraft systems.

**Plain definition:** Code that translates between external formats and
eddacraft's internal formats.

### MCP (Model Context Protocol)

A standard for providing context to AI models.

**Plain definition:** A protocol for tools to expose structured context to LLMs.
