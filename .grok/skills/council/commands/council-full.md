# /council-full

Run the full supervised Council review with debate resolution and synthesis. This
is the expensive compatibility/full-pack path; normal agentic delivery should
prefer risk-selected `council design` or `council assurance` lenses.

## Flow

```text
Resolve governing spec / immutable target
     ↓
[parallel] Run all configured reviewers
     ↓
[per reviewer] supervise output (max 2 retries)
     ↓
[contradictions] council-debate
     ↓
council-judge
     ↓
gate + decision + stop
```

## Usage

```text
/council-full
/council-full --target src/
/council-full --pr 123
/council-full --spec <path>
```

## Instructions

1. Resolve and pin the governing contract and target identity. Pass acceptance
   criteria and non-goals to every reviewer. Without a spec this is explicitly a
   standards-only review.
2. Run the configured full reviewer set in parallel. The legacy five-lens set is
   general, security, adversarial, operations, pragmatic; projects may add the
   maintainer lens when simplification/dependency discipline is material.
3. Supervise each result. Reject unsupported critical/major findings and any
   critical/major that is not `in_contract` when a spec exists. Retry a rejected
   reviewer at most twice.
4. Send approved deduplicated outputs to `council-judge`. If it returns
   `DEBATE_REQUIRED`, run `council-debate` for the material contradiction and
   judge once more with the debate result.
5. Present final JSON and **stop**. Do not implement and do not start another pack.

## Decision handling

The judge preserves `gate: PASS|WARN|BLOCK` for publication compatibility and
adds the loop-routing `decision`:

- `PASS` — no current-contract blocker. `gate` is PASS or WARN depending on advisories.
- `REPAIR` — small bounded defects; `gate: BLOCK` until repaired and re-verified.
- `REWORK` — implementation attempt is structurally poor/defect-dense; replace or
  escalate the implementation approach instead of patching every symptom.
- `REPLAN` — return to design/planning; do not keep repairing code against a bad contract.
- `BLOCK` — stop on unresolved critical risk/authority/environment/dependency.
- `DEBATE_REQUIRED` — resolve the contradiction before a final decision.

After a bounded `REPAIR`, deterministic evidence plus the independent verifier
check the change. Do not automatically run `/council-full` again. A new full pass
requires material redesign/scope change, a critical redesign finding, a new
material verifier concern, explicit policy, or explicit user request.

Use the ordinary Council skill for design-mode review, risk-selected assurance,
status, publication, and existing session-management operations.
