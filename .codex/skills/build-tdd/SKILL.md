---
name: build-tdd
description: >-
  Implement a ReadyItem with vertical-slice TDD: one failing test, minimal code,
  green, then next behaviour. Use for authorised implementation inside an
  isolated workspace. Return to plan-ready if expected behaviour is ambiguous.
---

# Build (TDD)

Implement only what the ReadyItem authorises. Prefer behaviour-through-public-
interfaces over implementation-coupled tests.

Borrowed: vertical slices / tracer bullets and "tests as specs" from mattpocock
`tdd`. Workflow-shaped for `dev-loop`, not a language tutorial.

## When

- ReadyItem is `ready` and workspace is isolated.
- `dev-loop` Execute / BUILD step.
- Bugfix with a reproducible failure (regression test first).

## Hard rules

1. **Vertical slices only.** One test → minimal code → next. Never write all
   tests then all code (horizontal slicing).
2. Tests assert **observable behaviour** via public interfaces, not private
   collaborators or call graphs.
3. If expected behaviour is ambiguous → **stop** and return to `plan-ready`
   (or `grill-design`). Do not invent scope.
4. Keep commits **focused**; include APS id in message when the item has one.
5. When test-first is impractical (generated code, pure config, spike), write a
   **replacement evidence note** before coding (what will prove the change, and
   why TDD was skipped).

## Steps

### 1. Confirm contract

Re-read ReadyItem: Expected behaviour + Validation commands. List behaviours to
cover in priority order (critical paths first — you cannot test everything).

### 2. Tracer bullet

```
RED:   one failing test for the first behaviour
       run it — must fail for the right reason
GREEN: minimal code to pass
       run it — must pass
```

### 3. Incremental loop

For each remaining behaviour: RED → GREEN. Only enough code for the current
test. Do not anticipate later tests.

### 4. Refactor (green only)

After a green bar: remove duplication, clarify names, deepen modules if natural.
Never refactor while red. Re-run targeted tests after each refactor step.

### 5. Regression on bugs

For fixes: reproduce with a failing test first; then fix; confirm red→green.

### 6. Commit cadence

After each meaningful green slice (or logical group), commit:

```text
feat(<scope>): <imperative summary>

APS: <ITEM-ID>
```

Omit `APS:` when there is no item id. Never commit secrets. Never `--no-verify`
unless the user explicitly orders it.

### 7. Unexpected failure

If tests fail for unclear reasons → `debug`, then resume BUILD.

## Exit

```markdown
## Exit

- Decision: implemented | needs-plan-update | blocked | validation-failed
- Next: evidence-gate | debug | plan-ready | stop
- Notes: <behaviours covered; TDD skip rationale if any>
```

Do not claim "done" here. `evidence-gate` then `verify-loop` own completion proof.

## Non-goals

- Not workspace setup (`isolate-workspace`).
- Not independent adversarial review (`verify-loop`).
- Not PR open (`land-branch`).
- Not generic Jest/framework tutorials — use project conventions.
