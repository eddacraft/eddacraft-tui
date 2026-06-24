---
name: verification-before-completion
description: Use when about to claim work is complete, fixed, or passing — before committing, creating PRs, or moving to the next task. Evidence before assertions, always.
---

# Verification Before Completion

**Core principle:** Evidence before claims. Always.

## The Iron Law

```
NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE
```

If you haven't run the verification command in this message, you cannot claim it passes.

## The Gate

Before any completion claim or expression of satisfaction:

1. **Identify** — what command proves this claim?
2. **Run** — execute it fresh, in full
3. **Read** — full output, exit code, failure count
4. **Verify** — does output confirm the claim?
5. **Only then** — make the claim, citing the evidence

Skip any step = lying, not verifying.

## Common Failures

| Claim            | Requires                    | Not sufficient              |
| ---------------- | --------------------------- | --------------------------- |
| Tests pass       | Test output: 0 failures     | Previous run, "should pass" |
| Linter clean     | Linter output: 0 errors     | Partial check               |
| Build succeeds   | Build: exit 0               | Linter passing              |
| Bug fixed        | Original symptom now passes | Code changed                |
| Requirements met | Line-by-line checklist      | Tests passing               |

## Red Flags — Stop

- Using "should", "probably", "seems to"
- Expressing satisfaction before verification ("Done!", "Perfect!", "Looks good!")
- About to commit without verifying
- Trusting a parallel task's success report without checking VCS diff
- "Just this once" / "I'm tired" / "Partial check is enough"

## Rationalisations

| Excuse                                  | Reality               |
| --------------------------------------- | --------------------- |
| "Should work now"                       | Run the verification  |
| "I'm confident"                         | Confidence ≠ evidence |
| "The other session said success"        | Verify independently  |
| "Different words so rule doesn't apply" | Spirit over letter    |

## When This Applies

Before:

- Any success/completion claim
- Any expression of satisfaction
- Any commit or PR
- Moving to the next task
- Aggregating results from parallel work streams
