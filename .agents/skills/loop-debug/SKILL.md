---
name: loop-debug
description: >-
  Evidence-based debugging for any unexpected failure: failing or flaky tests,
  regressions, confusing bugs, incidents — in or out of the development loop.
  Reproduce, isolate, hypothesize, fix the root cause, add regression coverage.
  Use before shotgun code changes.
---

# Debug

Find root cause with evidence. Do not guess-and-patch.

## When

- Tests fail unexpectedly during `loop-build-tdd` or `evidence-gate`.
- `dev-loop` repair path for non-obvious defects.
- User asks to debug a failure or incident.

## Hard rules

1. **Reproduce before fix.**
2. **One hypothesis at a time.**
3. Fix **root cause**, not symptoms.
4. Prefer a **regression test** that fails without the fix.
5. After three focused failed attempts → mark blocked / escalate to the lead
   (`dev-loop` repair budget), do not thrash.
   Blocked → stop; when running inside dev-loop, the lead consumes the
   Exit block.

## Steps

### 1. Reproduce

Exact steps, environment, consistency (always / intermittent). Minimise.

### 2. Isolate

Binary search (code, config, or `git bisect`). Find the boundary.

### 3. Analyse

Read stack traces, logs, recent diffs. Form 2–3 hypotheses with supporting and
contradicting evidence.

### 4. Test hypothesis

Predict outcome → run one experiment → record. Discard or promote hypothesis.

### 5. Fix

Implement the smallest correct fix. Add or adjust a regression test
(red without fix, green with fix) when feasible.

### 6. Verify

Re-run the original failure path and targeted suite. Hand back to
`evidence-gate` before any success claim.

## Exit

```markdown
## Exit

- Decision: fixed | blocked | needs-plan-update
- Next: loop-build-tdd | evidence-gate | plan-ready | stop
- Notes: <root cause one-liner; regression test path>
```

## Non-goals

- Not product design or scope expansion.
- Not full post-mortem process (optional short note only if asked).
