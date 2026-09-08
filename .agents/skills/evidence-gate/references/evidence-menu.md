# Evidence menu

Evidence is chosen to prove the claim at the ReadyItem risk class (see
[contracts.md](contracts.md), "Risk classes"). Collect what proves it — not
ceremony. Higher classes add categories; they never replace the base gates.

## Base (every class)

- validation commands from the ReadyItem, run fresh, full output read;
- focused tests for the changed surface;
- format, lint, and typecheck gates the repository mandates.

## Standard and above

- characterisation, unit, integration, contract, or end-to-end tests as the
  change surface demands;
- regression coverage for the fixed symptom path;
- negative-path and malformed-input tests where inputs cross a boundary.

## High

- permission, isolation, and concurrency tests where the change touches them;
- migration dry-run and rollback evidence for schema or data changes;
- deployment preview, smoke, and health evidence when the change is
  operationally material;
- reproducible commands, versions, inputs, and known limitations recorded.

## Critical

- calculation examples reconciled by a domain authority;
- backup, restore, and data-integrity evidence for anything destructive;
- performance, capacity, or failure-injection evidence where the risk is load
  or resilience;
- threat-model or security-review notes where the risk is security;
- explicit human approval recorded in the evidence block Notes.
