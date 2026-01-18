# REQUIREMENTS_AND_DECISIONS — Locked Inputs

## Product framing

- Anvil is AI ↔ human collaboration.
- Primary: trust in AI-generated code → faster safe shipping.
- Secondary: architecture compliance for human code.

## North-star outcomes (both)

1. More AI-generated code merged with confidence.
2. Drift slows or reverses over time.

## Core problem

Second-wave feature work drifts from intended patterns; biggest pain is
structural boundary crossing via new dependency edges.

## Posture

- Strong value without plans; codebase + dependency graph is the baseline.
- Primary loop: file save; PR/CI mirror later.

## Anti-patterns

- High-confidence warnings for AI escape hatches (eslint disables, `any`, broad
  ignores).
- Always explain + suggest.
- Suggestions: built-in library first; customisable; future AI enhancement.

## Exceptions

- Explicit suppression with human note.
- Note stored inline + provenance metadata.
- Humans approve exceptions/risk acceptance (never delegated to AI).

## Drift

Warn on NEW; acknowledge existing.

## Init

Exploratory; fallback descriptive entry points → internals map.

## Impact

Prefer runtime/product impact; ideally user journeys/features. Start
deterministically from runtime entry points; careful language; show
low-confidence explicitly only when low.
