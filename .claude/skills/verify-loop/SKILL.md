---
name: verify-loop
description: Independently verify an APS target, pull request, diff, branch, or completion claim against its governing specification and acceptance criteria. Use when assessing existing work, validating dev-loop output, deciding whether findings block completion, or re-verifying repairs; executor self-checks before claiming success are `evidence-gate`. Produces an evidence-backed binding or advisory decision without modifying implementation.
---

# Verification Loop

Verify the claim, not the executor's confidence. Remain read-only.

This is the independent verifier. The executor's own pre-claim self-check is
`evidence-gate`; an executor evidence block is input to this skill, never a
substitute for it.

## Inputs

Accept an APS target, PR, diff, branch, or completion claim. Resolve:

1. governing specification, APS plan/work item, accepted decisions, and repository policy;
2. bounded base and head revisions;
3. acceptance criteria, expected outcome, risk tier, and required gates;
4. the immutable **report reference** supplied by the lead, keyed by
   run, head revision, repair cycle, and attempt. The canonical logical form is
   `state://dev-loop/verify/<runId>/<head>/<cycle>/<attempt>/report.md`.
   The active harness adapter supplies its durable append/finalise operation; never
   substitute a generic file-writing tool or an unkeyed worktree path;
5. any executor evidence bundle offered for reuse. Treat it as an attestation,
   not authority; it is reusable only under the exact-match rule below.

If no governing contract or bounded change set can be established, return `under-specified`; never manufacture a pass.

## Independence

Start in a fresh context. Receive specification + plan + diff first. Do not receive executor reasoning, conversation history, implementation narrative, or self-assessment. After the blind pass, inspect relevant callers, tests, schemas, configuration, generated artefacts, and repository policy as needed.

Use an adversarial verifier role. For high-risk work and disputed findings, use a different model or harness where available. Never edit implementation; send repairs to the lead.

## Loop

1. **Map requirements.** Convert every acceptance criterion and binding policy into a checkable requirement.
   Apply specification precedence when sources conflict: accepted ADRs and
   repository policy outrank module specs, which outrank action plans / ReadyItems.
   If an action plan narrows parent scope without recording that narrowing, flag
   the parent-spec discrepancy even when the immediate ReadyItem passes.
2. **Inspect the change.** Review the bounded diff for omissions, unintended behaviour, unsafe assumptions, and policy violations.
3. **Inspect context.** Follow relevant integration surfaces beyond the diff without expanding into an unbounded repository review.
4. **Run fresh evidence.** Inspect the evidence bundle first. A completed full
   gate may be reused only when target, base/head, tree digest, dirty state,
   environment digest, governing sources, and command argv match exactly. Check
   its output digests and log references, then run fresh targeted/adversarial
   checks for the changed surface. If any identity field differs or logs are
   unavailable, execute the affected full gate. Read complete output and exit
   codes. If a read-only
   external-repository test mutates fixtures, writes caches outside the sandbox,
   or fails with `EROFS`, classify it as a tooling/sandbox failure unless product
   evidence remains after rerun in a hermetic writable environment.
5. **Create findings.** Each finding names severity, violated requirement, concrete evidence, reproduction where useful, and blocking status.
   **Record each check's outcome through the adapter's durable report operation
   as it completes** —
   findings, evidence, and exit codes, not intentions. The file is the
   crash-safe copy of the report: a run that dies mid-verification leaves a
   partial report the caller can read instead of nothing. Do not batch the
   whole report for the end; the end is exactly the moment a stall loses it.
   Finalisation must reject a head revision different from the one in the brief.
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

Emit an evidence bundle matching
`dev-loop/references/evidence-bundle.schema.json` (vended with dev-loop; if
unresolvable, record the limitation — the evidence bundle fields are also
summarised in `references/contracts.md`).
Never collapse absence of evidence into evidence of absence.

Treat missing negative tests as first-class verification output. When the change
touches parsing, rendering, escaping, auth, or trust boundaries, invent at least
one adversarial probe before accepting the executor's happy-path test set.

## Exit

The exit block and its findings are your **final message** — a run that ends
on anything else (a progress note, a next-step plan) has not exited, it has
stalled, and the caller receives the stall as your entire result. Restore
any bracketed experiment edits and show the clean workspace before the
report; once the report begins, run no further commands. Checks you did not
finish go in the report marked "not verified" — never silently dropped, and
never a reason to withhold the report the gathered evidence already
supports.

The contract is machine-checkable by design: the final message **ends with
the literal exit block below** — a `## Exit` heading and a `- Decision:`
line — and its content matches the report file. Callers detect a stall by
the absence of that block; do not paraphrase it, wrap it, or append
anything after it.

```markdown
## Exit

- Decision: pass | pass-with-advisories | repair-required | blocked | under-specified
- Next: land-branch | dev-loop | stop
- Notes: <advisory findings; blocking findings or blockers>
```
