---
name: dev-loop-differential
description: >-
  Optional multi-model/cross-harness routing overlay for the canonical agentic
  loop. Keeps the active native harness lead in authority and routes optional
  workers, advisors, and verifiers independently. Never changes scope or gates.
---

# Differential routing overlay

Load `agentic-loop`, `dev-loop`, and the selected native harness adapter first.
This skill changes role/model routing only.

## Invariants

- The active native harness **lead** retains scope, APS transitions, repair routing,
  PR lifecycle, and merge authority.
- A worker is optional; direct lead implementation remains valid even when
  differential mode is enabled.
- Advisor and verifier are independent roles and resolve separately.
- Different personas on the same provider do not count as provider diversity.
- Routing never weakens isolation, evidence, verification, or merge authority.
- Transcript transfer/import never counts as blind verification.

## Modes

- `off` — no external routing; do not probe or send repository material outside
  the active harness.
- `preferred` — try configured separation, then degrade explicitly to a fresh
  same-harness role when policy allows.
- `required` — required separation is a gate; stop `blocked` when unavailable.

One usable harness is sufficient for the ordinary loop.

## Role policy

APS `reasoning`, repository policy, and the work itself determine routing; do not
hard-code "fast worker, frontier everything else" as doctrine.

Useful defaults:

- routine mechanical delegated worker: `fast`;
- coherent complex implementation: lead or worker at `balanced`/`frontier` as
  the harness supports;
- architecture/risk advisor: `frontier` when judgement warrants it;
- acceptance verifier: strongest practical fresh model, different provider
  preferred for high/critical/disputed work when permitted.

Record requested and effective adapter/provider/model/effort and every degradation.

## Dispatch

Workers receive a bounded outcome, owned workspace/write surface, constraints,
acceptance evidence, and non-goals. Advisors/verifiers are read-only and receive
only the context required by their role.

Prefer native in-session or persistent harness capability. Cross-harness work may
use another native interactive/persistent session. Supervised headless transport is
an allowed fallback, not the default definition of differential routing.

## Failure

Unauthenticated, timed-out, malformed, permission-inadequate, or capability-
inadequate routes fail loudly. `preferred` tries the next candidate and records
fallback; `required` stops when the requirement cannot be met. Verification itself
never disappears.

## Finish

Return the ordinary loop outcome plus actual role routing, achieved independence,
and degradations.
