# Clawpatch Daily Workflow Brainstorm

| Type  | Authority | Owner | Status | Freshness |
| ----- | --------- | ----- | ------ | --------- |
| Spec  | Advisory  | CPTA  | Draft  | Captured 2026-05-17 from local `clawpatch doctor` and upstream README review |

| Upstream | Downstream |
| -------- | ---------- |
| [`plans/modules/clawpatch-techniques-adoption.aps.md`](../modules/clawpatch-techniques-adoption.aps.md), upstream `openclaw/clawpatch` README | Future CPTA discovery/spec work; optional Council workflow updates |

## Context

Anvil has a local `.clawpatch/config.json` and `clawpatch doctor` reports the
project state as OK with the Codex provider available. Clawpatch does not need a
separate OpenCode or Claude skill to function: the CLI already owns mapping,
review, reporting, triage, fix, and revalidation.

A skill may still be useful later if Anvil chooses to make clawpatch part of the
standard review lifecycle, but it should not become a new merge gate until the
signal-to-noise ratio is proven.

## Candidate Daily Uses

### 1. Morning Repo Health Scan

Run `clawpatch map`, then a small bounded review such as
`clawpatch review --limit 3 --jobs 3`.

Use this as an advisory scan for new or stale risk, especially maintainability,
test-gap, API-contract, and build-release findings. Do not treat this as a
blocking quality gate.

### 2. Pre-PR Second Opinion

After local validation passes and before Council, run clawpatch against the
branch. Fix obvious bugs or test gaps, triage false positives, then continue
with the normal `/council quick|mini|full` flow.

This is the best initial fit for Anvil because it complements Council without
replacing APS, TDD, or review gates.

### 3. Targeted Risk-Area Review

Use `clawpatch review --feature <id>` after mapping when work touches high-risk
areas such as Rust CLI commands, policy boundaries, release scripts, GitHub
workflows, persistence, or guard enforcement.

This keeps model spend focused and avoids whole-repo review noise.

### 4. Local Finding Backlog

Let `.clawpatch/findings` act as a local advisory backlog. Use
`clawpatch next`, `clawpatch show --finding <id>`, and `clawpatch triage` to
burn down findings opportunistically.

The backlog should not supersede APS. Any non-trivial fix still needs normal APS
authorisation or an explicitly approved hotfix path.

### 5. Regression-Test Input

Use a finding's `suggestedRegressionTest` as a TDD prompt. Write the failing
test manually, verify the red state, then implement the smallest fix.

This aligns clawpatch output with Anvil's TDD practice instead of allowing an
LLM-generated finding to become unverified implementation work.

### 6. Focused Revalidation

After fixing a finding, run `clawpatch revalidate --finding <id>` before broader
validation. Treat the result as a focused second check, not proof that the whole
branch is safe.

### 7. Advisory CI Experiment

Start with the `mock` provider to verify workflow shape in CI. If useful, trial
real provider reviews on scheduled jobs, labelled PRs, or non-blocking status
checks.

Avoid blocking CI on clawpatch until CPTA discovery proves reliable signal and a
clear remediation path.

### 8. Council Improvement Feed

Compare clawpatch findings with Council findings. If clawpatch repeatedly finds
valid categories Council misses, fold the prompt/schema pattern into Council
rather than adding a second mandatory review system.

## Recommended Starting Point

Start with two advisory habits:

1. **Pre-PR second opinion** for non-trivial branches.
2. **Targeted risk-area review** for high-risk file families.

Keep all output advisory until CPTA-001 confirms overlap, useful gaps, and the
right integration boundary with existing Council workflows.

## Skill Decision

No skill is required for ad hoc use. A future `clawpatch` skill would only be
worth adding if Anvil standardises operator behaviour around the CLI, such as:

- when to map and review
- which review limits and providers to use
- how to route findings into APS or Council
- when `fix` is allowed
- how revalidation evidence is recorded

Until then, `.clawpatch/config.json` plus direct CLI commands are enough.
