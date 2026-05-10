---
name: review
description: Targeted pre-PR review routed by changed paths and risk
---

# Targeted Review

## Target

$ARGUMENTS

## Purpose

`/review` is the lightweight pre-PR review entrypoint. It should produce focused
review evidence for the current change without running a full council by
default.

Use `/council mini` or `/council full` when deterministic guidance or the change
shape requires multiple reviewer roles.

Reference documents:

- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`
- `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- `plans/aps-rules.md`

## Routing

1. Resolve the target:
   - explicit file, glob, commit, or range from `$ARGUMENTS`
   - `staged` if staged changes exist
   - otherwise `recent` for the last commit
2. Run deterministic guidance when reviewing changed files:
   - staged: `scripts/agent/guidance.sh --staged --json`
   - branch or PR prep: `scripts/agent/guidance.sh --branch --json`
3. Select the reviewer role from the guidance and file paths.

Translate guidance output before reporting the review:

| Guidance value | Review value |
| --- | --- |
| `targeted` review tier | targeted pre-PR review |
| `mini` review tier | `/council mini` |
| `full` review tier | `/council full` |
| `council-reviewer` | `general` |
| `council-reviewer` with security-sensitive paths | `security` |
| `adversarial-reviewer` | `adversarial` |
| `operations-reviewer` | `operations` |
| `pragmatic-lead` | `pragmatic` |

## Reviewer Selection

| Trigger | Primary reviewer | Escalation |
| --- | --- | --- |
| normal code | `general` | `/council mini` for cross-boundary changes |
| docs-only | `general` if substantive | none |
| APS or planning process | `pragmatic` | planning council if scope/readiness changed |
| CI, release, deployment, workflow | `operations` | `/council mini` with `pragmatic` |
| auth, secrets, policy, trust boundary | `security` | `/council mini` with `adversarial` |
| edge-case-heavy or failure-path work | `adversarial` | `/council mini` with `general` |

Stable role names are `general`, `adversarial`, `operations`, `security`, and
`pragmatic`. Runtime agent IDs may differ by tool.

## Review Checklist

Focus on issues that affect merge safety:

- correctness and behavioural regressions
- missed edge cases or failure paths
- security and trust-boundary mistakes
- insufficient deterministic validation
- APS, documentation, or release authority drift
- missing tests or missing evidence for the change shape

## Output Format

```markdown
## Targeted Review: <target>

### Findings
- [severity] <category>: <description> — `<file>:<line>`
  Fix: <concrete action>

**Reviewer role:** <role>
**Verdict:** APPROVE | NEEDS CHANGES | ESCALATE TO COUNCIL

### Evidence Needed
- `<command>` or durable evidence reference

### Escalation
- None | `/council mini <target>` | `/council full <target>`
```

Findings must come first. If there are no findings, state that explicitly and
list residual risks or validation gaps.

## PR Evidence

Before opening a PR, summarise the review in the PR body or link to a durable
summary under `plans/reviews/` when the review is substantial.
