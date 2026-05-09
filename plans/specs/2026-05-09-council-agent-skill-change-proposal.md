# Council Agent And Skill Change Proposal

Date: 2026-05-09

Status: Proposed

Related spec:
`plans/specs/2026-05-09-plan-build-release-operating-model.md`

## Purpose

Review the current council agents, commands, and skills against the proposed
Plan / Build / Release operating model, then propose concrete changes.

The target is not more ceremony. The target is earlier, cheaper, more precise
agent review that prevents expensive CI failures, stale plans, and late release
rework.

## Relationship To The Operating Model

This proposal defines the review and council mechanics for the target operating
model in `2026-05-09-plan-build-release-operating-model.md`. It does not define a
parallel lifecycle.

Shared lifecycle vocabulary:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

Council and review sessions provide judgement evidence at specific transitions:

| Transition | Review role |
| --- | --- |
| Draft / Proposed plan creation | Planning council creation or direction validation. |
| Ready -> In Progress for non-trivial work | Planning council pre-execution validation or documented lightweight exception. |
| Local work -> PR | Targeted pre-PR review, mini council, or full council based on risk. |
| PR -> Merged | PR review, CI, and any triggered council escalation. |
| Merged -> Released/Shipped | Release candidate and verification checks; council only for risk-triggered release/process changes. |

Review terminology is canonical across the new specs:

| Term | Definition |
| --- | --- |
| Targeted review | One selected reviewer role, usually pre-PR. |
| Mini council | Two selected reviewer roles for elevated risk. |
| Full council | Formal multi-reviewer review for system-changing work. |
| Planning council | Plan creation, direction validation, or pre-execution reality validation. |

Review findings are not validation proof. They are structured judgement evidence
that can require deterministic checks, APS amendments, or implementation changes.
CI remains validation authority for commit SHAs.

Hooks may print deterministic review guidance, but hooks must not run LLM review.

## Current Council Surfaces Reviewed

Repository-local surfaces:

- `.claude/commands/council.md`
- `.claude/commands/review.md`
- `.claude/commands/plan.md`
- `.claude/agents/council-reviewer.md`
- `.claude/agents/adversarial-reviewer.md`
- `.claude/agents/operations-reviewer.md`
- `.claude/agents/pragmatic-lead.md`
- `.claude/agents/plan-synthesizer.md`
- `.claude/agents/anvil-plan-spec.md`
- `.claude/agents/protocols.md`
- `.claude/council/schema.json`
- `.claude/council/council-session.sh`
- `.claude/council/council-publish.sh`
- `.claude/hooks/codex-review-post.sh`

Global skills reviewed conceptually through loaded skill content:

- `council`
- `local-review-council`
- `planning-council`

Related existing APS module:

- `plans/modules/council-gate-bridge.aps.md`

## Current-State Assessment

### Strengths

- There is already a local council session store under `.claude/council/`.
- Council findings have structured severity, status, evidence, waivers, and
  publication support.
- `planning-council` already models interrogation, negotiation, synthesis, and
  review.
- `plan-synthesizer` already understands how to produce ADRs, APS modules, and
  index updates.
- `anvil-plan-spec` already treats work items as execution authority.
- The global `council` skill already distinguishes quick, standard, and full
  reviewer packs.

### Gaps

- The repo-local `/council` command still assumes all five reviewers should run
  for normal council use. It does not reflect the proposed risk-tier model.
- The repo-local `/review` command is generic and does not route to targeted
  reviewers by changed paths or risk area.
- `planning-council` is not integrated into APS readiness or pre-execution
  gates.
- `anvil-plan-spec` allows execution after status/dependency checks, but does
  not require a current planning-council reality check before substantial work.
- Council session schema models review sessions, but not planning validation
  sessions as first-class lifecycle records.
- Hook behaviour currently includes a post-commit Codex review path. That is too
  late for the desired pre-PR review loop and risks becoming noisy commentary.
- There is no deterministic path-to-playbook/reviewer/check rules engine shared
  by hooks, agents, and CI.
- Council outputs are not yet tied cleanly to PR metadata, APS status, release
  candidates, or release records.
- Agent names are inconsistent across surfaces: repo-local agents use names such
  as `council-reviewer`; global OpenCode subagents use names such as
  `council-general`.

## Target Council Model

Council should operate at four points:

```text
planning creation/validation
  -> pre-execution plan reality check
  -> pre-PR targeted review
  -> PR-level risk council only when triggered
```

Review timing:

```text
precommit = deterministic guidance only
pre-PR = targeted agent review by default
PR = CI + human review + risk-triggered council
post-merge = drift and release-readiness checks
```

Council is a risk-control tool, not a universal ceremony gate.

## Recommended Artefact Changes

### 1. Refactor Release Skill Into Router Plus Playbooks

Current issue: `.claude/skills/release/SKILL.md` is one long process document.

Recommended structure:

```text
.claude/skills/release/
  SKILL.md
  playbooks/
    00-route.md
    10-candidate.md
    20-version-and-notes.md
    30-readiness.md
    40-publish.md
    50-verify.md
    60-closeout.md
    70-patch.md
    80-rollback.md
    90-recovery.md
  references/
    artefact-list.md
    release-state-model.md
    versioning-rules.md
    github-commands.md
  schemas/
    release-candidate.schema.json
    release-record.schema.json
```

`SKILL.md` should route to playbooks, not carry every step inline.

### 2. Add Planning Council Playbooks

Recommended structure:

```text
.claude/skills/planning-council/
  SKILL.md
  playbooks/
    plan-create.md
    direction-validate.md
    pre-execution-validate.md
    plan-amend.md
    plan-synthesize.md
  checklists/
    repo-reality-check.md
    aps-readiness-check.md
    as-built-docs-check.md
```

Purpose:

- Creation council for new plans.
- Direction validation before a draft becomes execution authority.
- Pre-execution validation before non-trivial Ready work starts.
- Amendment loop when repo reality invalidates the plan.

### 3. Add Deterministic Agent Guidance Script

Add a rules engine callable by hooks, agents, and CI:

```text
scripts/agent-guidance.sh --staged
scripts/agent-guidance.sh --branch
scripts/agent-guidance.sh --pr
```

Recommended output:

```json
{
  "requiredPlaybooks": [
    ".claude/skills/release/playbooks/release-workflow-change.md"
  ],
  "requiredReviews": ["Operations", "Pragmatic Lead"],
  "requiredChecks": ["release-readiness"],
  "warnings": [
    "release workflow changed; PR-level mini council required"
  ]
}
```

Hooks should print this guidance. Agents should consume it. CI should enforce
stable mandatory pieces.

### 4. Replace Post-Commit Review With Pre-PR Guidance

Current `codex-review-post.sh` reviews after successful commits. That is later
than the desired quality loop.

Recommended change:

- Keep post-commit review disabled by default or retire it.
- Add deterministic precommit or pre-push guidance that tells the agent which
  playbook to read and which review tier applies.
- Do not run LLM review inside Git hooks.

Hook output should look like:

```text
Agent guidance:
Release process files changed.

Before opening PR:
- Read: .claude/skills/release/playbooks/release-workflow-change.md
- Review: Operations + Pragmatic Lead
- Check: release-readiness impact
- Include in PR: release impact + rollback note
```

### 5. Update `/council` To Support Review Tiers

Repo-local `.claude/commands/council.md` should support:

```text
/council quick <target>       # one targeted reviewer
/council mini <target>        # two selected reviewers
/council full <target>        # full panel
/council publish             # PR-ready summary
/council status              # latest session state
```

Default should be quick/targeted, not all five reviewers.

Recommended tier mapping:

| Change Shape | Default Council Mode |
| --- | --- |
| Small code change | quick |
| Medium one-subsystem change | quick, plus adversarial if risky |
| Cross-boundary change | mini |
| Security/auth/policy/release/CI | mini or full |
| Branch/release/workflow model | full |

### 6. Update `/review` To Become Targeted Pre-PR Review

Repo-local `.claude/commands/review.md` should stop being a generic checklist
only. It should route based on changed paths and risk:

- General reviewer for normal code.
- Operations reviewer for CI/release/deployment.
- Security reviewer for auth/secrets/policy.
- Adversarial reviewer for edge-case-heavy changes.
- Pragmatic lead for scope/process/planning changes.

It should output PR-ready review evidence or a council session reference.

### 7. Extend Council Session Schema

Current schema supports review sessions. Extend it to support planning and
pre-execution validation records.

Suggested additions:

```json
{
  "sessionKind": "code-review|planning|pre-execution|release",
  "apsItems": ["MOD-001"],
  "playbooks": [".claude/skills/planning-council/playbooks/pre-execution-validate.md"],
  "repoReality": {
    "baseBranch": "main",
    "headSha": "...",
    "changedSincePlan": true
  },
  "decision": "proceed|amend|split|replan|block"
}
```

This gives agents a durable record they can reference before execution and in PR
summaries.

### 8. Update `anvil-plan-spec` Execution Rules

Current execution checks:

- locate work item
- verify Ready
- verify dependencies
- read spec
- create action plan if needed
- execute and validate

Recommended additional step before execution:

```text
Run planning-council pre-execution validation for non-trivial Ready work unless
a current validation record exists and repo reality has not materially changed.
```

If validation returns `amend`, `split`, `replan`, or `block`, the APS agent must
stop execution and update the plan first.

### 9. Add As-Built Documentation Gate Later

The user has recently overhauled as-built style docs and wants a separate
session on that system. Do not prematurely lock the exact rules here.

Reserve an integration point now:

- Planning Council pre-execution validation should ask whether as-built docs are
  affected.
- PR metadata should include as-built docs status when relevant.
- Release candidate generation should include docs completeness once the docs
  model is finalised.

Downstream artefact needed:

```text
plans/specs/YYYY-MM-DD-as-built-docs-integration.md
```

### 10. Align Agent Names Across Claude And OpenCode

Current mismatch:

- Claude repo-local: `council-reviewer`, `operations-reviewer`,
  `adversarial-reviewer`, `pragmatic-lead`
- OpenCode global: `Council — General`, `Council — Operations`, etc.

Recommended stable role names:

- `general`
- `adversarial`
- `operations`
- `security`
- `pragmatic`
- `planning-synthesizer`

Commands and schemas should store role names separately from runtime agent IDs.

Example:

```json
{
  "role": "operations",
  "runtime": "claude",
  "agentId": "operations-reviewer"
}
```

## Proposed Review Trigger Rules

| Trigger | Pre-PR Review | PR Escalation |
| --- | --- | --- |
| docs-only | none or General if substantive | none |
| APS plan creation | Planning Council direction validation | full planning review if architecture changes |
| APS Ready item execution | Planning Council pre-execution validation | none unless validation changes scope |
| release skill/runbook/workflow | Operations + Pragmatic | mini or full council |
| branch/release operating model | Operations + Pragmatic | full council |
| CI scripts/workflows | Operations | Operations + Pragmatic |
| auth/secrets/policy | Security | Security + Adversarial |
| cross-boundary code | General + Pragmatic | mini council |
| urgent patch | targeted reviewer | post-release council if rushed |

## Proposed Implementation Order

1. Update operating spec with Planning Council gates.
2. Create planning-council playbook stubs.
3. Refactor release skill into router plus playbooks.
4. Add deterministic `agent-guidance` script in advisory mode.
5. Update `/review` and `/council` command docs for tiered review.
6. Extend council schema for session kind, APS items, playbooks, and decision.
7. Update APS agent rules to require pre-execution validation for non-trivial
   Ready work.
8. Add CI warning check for missing review guidance metadata.
9. Later: integrate as-built docs once that system is reviewed.

## Recommendation

Adopt this direction.

The key design choice is to keep hooks deterministic and cheap while making them
point agents at the correct playbooks and review tiers. Agents should execute
playbooks explicitly at pre-PR, planning, release-candidate, publish, and verify
stages. Hooks should not run LLMs.
