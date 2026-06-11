---
name: council
description:
  Risk-tiered Council review. Runs quick, mini, or full review based on target
  risk, then publishes PR-ready evidence.
---

# Council Review

## Target

$ARGUMENTS

## Purpose

`/council` is the repo-local entrypoint for judgement evidence before PRs and
for risk-triggered PR escalation. It follows the operating-model review tiers in
`plans/specs/2026-05-09-council-agent-skill-change-proposal.md`.

Council findings are review evidence, not validation proof. CI and local checks
remain validation authority.

## Usage

```text
/council [quick|mini|full] <target>
/council status
/council publish
```

Targets may be file paths, globs, commit refs, branch ranges such as
`main...HEAD`, `staged`, `recent`, or empty. Empty defaults to `staged` when
staged changes exist, otherwise `recent`.

## Review Tiers

| Tier | Reviewers | Use |
| --- | --- | --- |
| `quick` | one selected reviewer | Default for normal pre-PR review. |
| `mini` | two selected reviewers | Cross-boundary, CI, release, security, or workflow risk. |
| `full` | all reviewer roles | Branch/release operating-model changes or high-risk design changes. |

Default to `quick` unless the target or `scripts/agent/guidance.sh` indicates a
higher tier.

## Role Map

Store stable role names in summaries; runtime agent IDs may differ by tool.

| Role | Claude agent | Focus |
| --- | --- | --- |
| `general` | `council-reviewer` | Correctness, maintainability, test coverage. |
| `adversarial` | `adversarial-reviewer` | Edge cases, failure paths, abuse cases. |
| `operations` | `operations-reviewer` | CI, release, deployment, observability, recovery. |
| `security` | `council-reviewer` | Secrets, auth, policy, injection, trust boundaries. |
| `pragmatic` | `pragmatic-lead` | Proportionality, scope, ship-readiness. |

## Guidance Translation

`scripts/agent/guidance.sh` is the deterministic source for changed-path risk,
but it currently emits migration-state names. Translate them before reporting
Council state:

| Guidance value | Council value |
| --- | --- |
| `targeted` review tier | `quick` council tier |
| `mini` review tier | `mini` council tier |
| `full` review tier | `full` council tier |
| `council-reviewer` | `general` |
| `council-reviewer` with security-sensitive paths | `security` |
| `adversarial-reviewer` | `adversarial` |
| `operations-reviewer` | `operations` |
| `pragmatic-lead` | `pragmatic` |

## Routing

1. Resolve the requested tier and target.
2. Run `scripts/agent/guidance.sh --staged`, `--branch`, or `--files-from` when a
   changed-file list is available.
3. Use guidance output to select reviewers:
   - release, CI, workflow, or branch/release model: `operations` + `pragmatic`
   - auth, secrets, policy, or untrusted input: `security` + `adversarial`
   - cross-boundary source changes: `general` + `pragmatic`
   - normal code or docs: `general`
4. Escalate to `full` for branch/release operating-model changes or when a
   reviewer returns critical/major findings that need multi-role judgement.

## Commands

### `quick`

Run one targeted reviewer. Use this as the normal pre-PR council pass.

### `mini`

Run two reviewers selected by risk. Use this for elevated risk before PR or when
CI/release/security/workflow files changed.

### `full`

Run all roles: `general`, `adversarial`, `operations`, `security`, and
`pragmatic`. Use this sparingly for operating-model, release, branch, and other
system-changing work.

### `status`

Report the current in-chat or local review session state if one exists: target,
tier, open findings, resolved findings, evidence, and publish status. The
workflow session/event schema landed under OPMODEL-009
(`plans/specs/2026-05-10-workflow-session-and-event-schema.md`,
`schemas/workflow-session-event.v1.schema.json`), but the durable session store
that would back `status` is still owned downstream by CGBDG and is not yet
wired up — `status` must not imply a repository-backed session store. If no
current session exists, report that explicitly.

### `publish`

Produce a PR-ready summary from the current converged review. Until CGBDG
wires the OPMODEL-009 workflow session schema into a repository-backed store,
the source is the current chat/local review state or an explicitly provided
review file under `plans/reviews/`.

```markdown
## Council Review

### Findings
- <severity> <role>: <finding> — `<file>:<line>`

**Status:** Converged
**Tier:** quick | mini | full
**Target:** <target>

### Evidence
- `<command>`
```

Write durable summaries under `plans/reviews/` when the review is substantial or
when the PR body needs a stable reference.

## Output Rules

- Findings first, ordered by severity.
- Include file and line references.
- Critical and major findings must be fixed, explicitly deferred, or waived with
  rationale before PR.
- Do not run LLM review from Git hooks; hooks may only print deterministic
  guidance.
