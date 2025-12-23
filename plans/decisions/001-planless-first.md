# ADR-001: Planless-first Posture

## Status

Accepted

## Context

Anvil originated as a plan validation and execution tool, requiring users to
write APS planning documents before getting value. This created adoption
friction — developers had to learn a new format before seeing any benefit.

## Decision

Anvil v1 delivers value **without requiring plans**. The baseline source of
truth (when no plan exists) is the current codebase and its dependency
structure.

Plans and APS remain valuable as an accelerant and governance layer, but are not
a prerequisite for the core warning functionality.

## Consequences

- Lower adoption barrier — install and run immediately
- Architecture inference must work without explicit configuration
- Planning features become "advanced" rather than "required"
- Marketing/docs must not lead with APS complexity
