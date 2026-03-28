<!--
APS Module: Temper Workflow
============================
GitHub Actions self-healing workflow that auto-addresses
CI review comments with a hard cap on cycles.
See: plans/aps-rules.md
-->

# Temper Workflow

| ID     | Owner  | Status   |
| ------ | ------ | -------- |
| TEMPER | @aneki | Complete |

## Purpose

Implement the GitHub Actions workflow (`temper.yml`) that auto-addresses CI
review comments posted on PRs. The workflow runs up to 2 self-healing cycles:
cycle 1 addresses all fixable findings, cycle 2 only addresses findings on
lines changed by cycle-1 fixes. Everything remaining after cycle 2 is deferred
to issues. The workflow supports both automatic triggering (on review comments
when `forge:tempered` label is present) and manual dispatch.

## In Scope

- `temper.yml` GitHub Actions workflow with auto and manual trigger modes
- Cycle 1: fetch unresolved threads, categorize (fix/reply/defer), apply fixes,
  resolve threads, push, post summary
- Cycle 2: scoped re-review (only lines changed in cycle 1), defer everything
  else, push, post final summary
- Hard cap at 2 cycles enforced by workflow counter
- `forge:tempered` label gating for auto-trigger mode
- `workflow_dispatch` manual trigger with PR number input
- Summary comments posted on PR after each cycle
- Bot mention avoidance (no `@copilot`, `@coderabbitai`)

## Out of Scope

- The Forge pre-commit phase (Module 1: FORGE)
- Finding filing logic (Module 3: DEFER -- but Temper calls it)
- The negotiation protocol (Module 2: FNEG)
- Modifying existing CI review bot configurations
- Auto-merging PRs (human always gates the final merge)

## Interfaces

**Depends on:**

- DEFER module — filing deferred findings as issues
  (Note: DEFER also lists TEMPER as a dependency — this is a mutual
  runtime dependency. TEMPER calls DEFER to file remaining findings;
  DEFER accepts findings from TEMPER as input. DEFER was implemented
  first; TEMPER consumes it.)
- GitHub GraphQL API — fetching unresolved review threads
- GitHub Actions — workflow runtime
- `CLAUDE_TEMPER_ENABLED` / `CLAUDE_TEMPER_MAX_CYCLES` — repo-level variables

> **Note:** TEMPER depends on DEFER for filing, while DEFER lists TEMPER as an
> input source. This is not circular at runtime — DEFER is a utility invoked by
> Temper via shell commands during the filing step. DEFER was built first.

**Exposes:**

- `.github/workflows/temper.yml` — the self-healing workflow
- PR summary comments — posted after each cycle
- `forge:tempered` label — gate for automatic triggering

## Constraints

- Maximum 2 cycles -- hard cap, no configuration to exceed this
- Cycle 2 is scoped to changes from cycle 1 only
- No bot mentions in comments or thread replies
- `CLAUDE_TEMPER_ENABLED` only controls automatic trigger; manual dispatch always
  works
- Workflow must not modify files outside the PR's scope
- All deferred findings must be filed (via DEFER module) before workflow completes

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (DEFER, GH GraphQL, GH Actions)
- [x] All tasks defined
- [x] Trigger modes documented in design doc

## Tasks

### TEMPER-001: Create temper.yml workflow scaffold

- **Status:** Complete
- **Intent:** Establish the GitHub Actions workflow file with both trigger modes
  and cycle management infrastructure
- **Expected Outcome:** Workflow file with `pull_request_review` and
  `workflow_dispatch` triggers, cycle counter, label check for auto mode, and
  job structure for cycle 1 and cycle 2
- **Validation:** Workflow passes GitHub Actions syntax validation and appears in
  the Actions tab
- **Files:** `.github/workflows/temper.yml`
- **Confidence:** high

### TEMPER-002: Implement cycle 1 -- full review addressing

- **Status:** Complete
- **Intent:** The first cycle fetches all unresolved review threads, categorizes
  them, applies fixes, and resolves threads
- **Expected Outcome:** Cycle 1 fetches unresolved threads via GraphQL,
  categorizes each as fix/reply/defer based on severity, applies fixes and
  commits, replies to threads, resolves all addressed threads, and pushes
- **Validation:** After cycle 1, previously unresolved threads on the PR are
  resolved and a summary comment is posted
- **Dependencies:** TEMPER-001
- **Confidence:** medium

### TEMPER-003: Implement cycle 2 -- scoped re-review

- **Status:** Complete
- **Intent:** The second cycle only addresses findings on lines changed by
  cycle-1 fixes, deferring everything else
- **Expected Outcome:** Cycle 2 computes the diff from cycle-1 commits, filters
  new findings to only those touching changed lines, applies fixes, defers all
  remaining findings to issues, posts final summary
- **Validation:** After cycle 2, a summary comment states "Temper complete" and
  lists all deferred findings with issue links
- **Dependencies:** TEMPER-002
- **Confidence:** medium

### TEMPER-004: Implement cycle cap enforcement

- **Status:** Complete
- **Intent:** The workflow never runs more than 2 cycles regardless of new
  comments
- **Expected Outcome:** A cycle counter tracks iterations. If the workflow is
  triggered again after cycle 2 completes, it posts a comment explaining the cap
  was reached and exits without making changes
- **Validation:** Triggering the workflow a third time on the same PR results in
  a "cap reached" comment and no code changes
- **Dependencies:** TEMPER-001
- **Confidence:** high

### TEMPER-005: Implement manual dispatch trigger

- **Status:** Complete
- **Intent:** Allow manual triggering of Temper on any PR regardless of label or
  toggle state
- **Expected Outcome:** `workflow_dispatch` accepts a PR number input, checks out
  the PR branch, and runs the same cycle logic as auto mode
- **Validation:** Running the workflow manually via Actions UI with a PR number
  triggers cycle 1 on that PR
- **Dependencies:** TEMPER-001
- **Confidence:** high

### TEMPER-006: Implement PR summary comments

- **Status:** Complete
- **Intent:** Each cycle posts a structured summary comment on the PR
- **Expected Outcome:** Summary includes: findings addressed (with categories),
  findings deferred (with issue links), cycle number, and next steps
- **Validation:** After each cycle, the PR has a new comment from the workflow
  with a structured summary
- **Dependencies:** TEMPER-002, TEMPER-003
- **Confidence:** high
