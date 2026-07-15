---
name: verify-loop
description: Independently verify an APS target, pull request, diff, branch, or completion claim against its governing specification and acceptance criteria. Use when assessing existing work, validating dev-loop output, deciding whether findings block completion, or re-verifying repairs. Produces an evidence-backed binding or advisory decision without modifying implementation.
---

# Verification Loop

Verify the claim, not the executor's confidence. Remain read-only.

## Inputs

Accept an APS target, PR, diff, branch, or completion claim. Resolve:

1. governing specification, APS plan/work item, accepted decisions, and repository policy;
2. bounded base and head revisions;
3. acceptance criteria, expected outcome, risk tier, and required gates.

If no governing contract or bounded change set can be established, return `under-specified`; never manufacture a pass.

## Independence

Start in a fresh context. Receive specification + plan + diff first. Do not receive executor reasoning, conversation history, implementation narrative, or self-assessment. After the blind pass, inspect relevant callers, tests, schemas, configuration, generated artefacts, and repository policy as needed.

Use an adversarial verifier role. For high-risk work and disputed findings, use a different model or harness where available. Never edit implementation; send repairs to the orchestrator.

## Loop

1. **Map requirements.** Convert every acceptance criterion and binding policy into a checkable requirement.
   Apply specification precedence when sources conflict: accepted ADRs and
   repository policy outrank module specs, which outrank action plans / ReadyItems.
   If an action plan narrows parent scope without recording that narrowing, flag
   the parent-spec discrepancy even when the immediate ReadyItem passes.
2. **Inspect the change.** Review the bounded diff for omissions, unintended behaviour, unsafe assumptions, and policy violations.
3. **Inspect context.** Follow relevant integration surfaces beyond the diff without expanding into an unbounded repository review.
4. **Run fresh evidence.** Execute plan validation, focused tests, repo gates,
   and risk-specific checks. Read complete output and exit codes. If a read-only
   external-repository test mutates fixtures, writes caches outside the sandbox,
   or fails with `EROFS`, classify it as a tooling/sandbox failure unless product
   evidence remains after rerun in a hermetic writable environment.
5. **Create findings.** Each finding names severity, violated requirement, concrete evidence, reproduction where useful, and blocking status.
6. **Decide.** Apply risk-tiered authority:
   - objective gate failures block;
   - high-confidence critical and major findings block;
   - minor and subjective design concerns are advisory unless policy elevates them;
   - disputed material findings require differential or Council review.
7. **Re-verify.** After repair, verify the new bounded diff against the original contract and open findings. Do not accept an executor's claim that a finding is fixed.

## Decisions

- `pass` — all binding requirements have fresh supporting evidence.
- `pass-with-advisories` — binding requirements pass; non-blocking findings remain.
- `repair-required` — one or more binding findings remain.
- `blocked` — verification cannot run because of access, environment, dependency, or authority.
- `under-specified` — no adequate governing contract or bounded change set exists.

Emit an evidence bundle matching the installed pack schema. Prefer
`dev-loop-core/references/evidence-bundle.schema.json` when verifying
`dev-loop-core`; otherwise use `dev-loop/references/evidence-bundle.schema.json`.
Never collapse absence of evidence into evidence of absence.

Treat missing negative tests as first-class verification output. When the change
touches parsing, rendering, escaping, auth, or trust boundaries, invent at least
one adversarial probe before accepting the executor's happy-path test set.
