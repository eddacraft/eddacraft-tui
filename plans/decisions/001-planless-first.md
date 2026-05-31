# ADR-001: Planless-first Posture

## Status

Accepted

## Note

This is a **product principle (tool UX)**, not an agent feature-evaluation gate.
It describes Anvil's zero-config posture: the tool delivers value without the
user writing a plan. It is **not** a per-feature constraint for agents to
validate work against, and "no APS item exists" is unrelated to it. It marks the
pivot away from the original spec-driven-validation thesis (parse plan →
translate to APS → block code on divergence), which is retired.

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
