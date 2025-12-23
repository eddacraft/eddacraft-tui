# ADR-003: New Edges Only

## Status

Accepted

## Context

Most codebases have existing architecture violations — legacy imports, circular
dependencies, boundary crossings. Warning on all of them would overwhelm
developers and make the tool unusable.

## Decision

Anvil baselines existing architecture on first run. Warnings are generated only
for **new** violations introduced after the baseline.

Existing violations are tracked for drift reports but do not generate save-time
warnings.

## Consequences

- No "wall of warnings" on adoption
- Focus on preventing new drift, not fixing all legacy issues
- Baseline must be generated and stored (`.anvil/baseline.json`)
- Risk: baseline could include recent bad code
- Mitigation: baseline can be regenerated; drift reports show existing counts
