# Shared contracts for dev-loop core

All pack skills use these shapes. Do not invent parallel handoff formats.

## ReadyItem

The authorised unit of work. Produced by `plan-ready`. Consumed by isolate → build → verify → land.

```markdown
## ReadyItem

- Goal: <one sentence outcome>
- Work item: <APS ID or ad-hoc:<slug>>
- Status: Ready
- Expected behaviour: <observable outcomes>
- Files: <paths or globs known today>
- Validation commands: <exact commands, one per line>
- Dependencies: <none | IDs with status>
- Risk: low | standard | high | critical
- Design source: <path or "none">
- Constraints / non-goals:
- PR base: <integration | dependency-branch> (default integration)
- Stack depends on: <none | #PR branch@sha>
- Decision: ready | needs-design | blocked | out-of-scope | needs-plan-update
```

Rules:

1. **Validation commands are required** for `ready`. No "TBD".
2. Prefer current project truth (code, tests, CI) over stale plan prose.
3. One primary work item per ReadyItem. Split multi-module goals first.
4. In APS projects, the ReadyItem must match a real item after `aps-planning` truth validation.
5. Lifecycle vocabulary (canonical):
   `Draft → Proposed → Ready → In Progress → Merged → Released/Shipped → Complete`
   (`Blocked` is a side-state. `Committed` is legacy for `Merged`.)

## Risk classes

The ReadyItem `Risk` field binds required controls, not just labels. Evidence
is chosen to prove the claim at the chosen class — collect what proves it, not
ceremony (see `evidence-gate` references/evidence-menu.md).

| Risk     | Typical work                                                                                   | Required control                                                                                |
| -------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| low      | Local, reversible, familiar, low blast radius                                                   | Validation commands pass                                                                           |
| standard | Default for feature work                                                                        | Validation commands, tests for changed behaviour, review                                           |
| high     | Cross-boundary, operationally material, sensitive, or hard to observe                           | Explicit design source, broader evidence, rollback noted, independent `verify-loop`                |
| critical | Domain-critical calculation, tenant isolation, security-sensitive, destructive, or irreversible | All of high, plus differential design where triggered, domain authority, explicit human approval |

When uncertain, choose the higher class until evidence justifies lowering it.
Raising risk needs no permission; lowering it requires a recorded human
decision.

## Evidence block (executor)

Produced by `evidence-gate` before any success claim, land, or handoff to `verify-loop`.

```markdown
## Evidence

- Target: <ReadyItem id>
- Claim: <what is being claimed>
- Commands:
  - `<cmd>` → exit <n> — <one-line summary>
- Classification: product-failure | tooling-environment | inherited-baseline | pass
- Base..head: <sha>..<sha> (if known)
- Result: supported | not-supported
- Notes: <gaps, inherited failures, skipped gates with reason>
```

Rules:

1. Fresh run in this turn — prior logs do not count.
2. Read full output and exit codes; do not summarise from memory.
3. Absence of a run is not a pass.
4. Tooling/sandbox/environment failures are not product failures until rerun in a
   hermetic writable environment still proves a product defect.

Independent adversarial verification uses `verify-loop` and
`dev-loop/references/evidence-bundle.schema.json`. The executor evidence
block is necessary but not sufficient for high-risk or autonomous land.

## Stage exit

Every pack skill ends with:

```markdown
## Exit

- Decision: <enum for this skill>
- Next: <skill name or stop>
- Notes: <optional one line>
```

## Who owns what

| Concern                                    | Owner                              |
| ------------------------------------------ | ---------------------------------- |
| Orchestration, claims, repair budget, mode | `dev-loop`                         |
| Intent → ReadyItem                         | `plan-ready` (+ `grill-design`)    |
| APS load / truth / reconcile               | `aps-planning`                     |
| Workspace + branch                         | `isolate-workspace`                |
| Implementation                             | `loop-build-tdd`                        |
| Unexpected failures                        | `loop-debug`                            |
| Executor self-check                        | `evidence-gate`                    |
| Independent verification                   | `verify-loop`                      |
| PR / merge / cleanup                       | `land-branch`                      |
| PR feedback + CI after open                | `address-reviews`                  |
| Multi-persona review                       | `council`                          |
