---
id: kindling
title: Signal Capture (Kindling)
description:
  Capture development observations with enough context for later reuse.
sidebar_position: 1
owner: DOCSYNC
---

# Signal Capture (Kindling)

Kindling is the capture capability in the memory system. It records useful
development observations at the moment they occur.

## Why this capability matters

- Prevents knowledge loss between sessions
- Reduces repeated debugging and rediscovery
- Creates evidence for later review and promotion

## What good capture looks like

Capture entries that can help someone else act faster later:

- Root causes and failure modes
- Constraints and non-obvious behaviour
- Architectural decisions with rationale
- Operational gotchas and workarounds

## Capture quality rules

- Write what happened, not just that something was "broken"
- Include context (component, API, command, environment)
- Keep one observation per entry for easier reuse

## Example

```bash
kindling observe "Webhook validation fails if request body is parsed before signature check"
```

This is high-signal because it includes the trigger and the consequence.

## How it connects to the rest of the system

- Captured observations feed candidate review in Ember.
- Reviewed candidates become canonical memory in Edda.
- Provenance chain stays intact across layers.

## Operating contract

- **Inputs:** Live development observations, debugging outcomes, non-obvious
  constraints
- **Decision point:** Is this worth preserving for future reuse?
- **Outputs:** Structured, searchable observations with context
- **Evidence:** Timestamped capture record linked to session context

## Current role in adoption

If you adopt only one part of the system first, start here. Capture delivers
immediate value with minimal process change.

[Full Kindling documentation →](/kindling/overview)

---

**Next:** [Candidate Review (Ember) →](/edda-stack/components/ember)
