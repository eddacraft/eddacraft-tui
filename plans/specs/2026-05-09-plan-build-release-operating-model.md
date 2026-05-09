# Plan / Build / Release Operating Model Specification

Date: 2026-05-09

Status: Proposed

Source review: `review/plan-build-release-operating-model.md`

## Purpose

Define the target Plan / Build / Release workflow for Anvil.

The workflow optimises for the fastest possible path from validated intent to
safe production release. It is designed for an AI-native operating model where
agents and humans both need deterministic, low-friction, mechanically checked
state transitions.

This specification is the anchor artefact. It is expected to produce additional
implementation artefacts as decisions harden: runbooks, CI workflows, APS schema
updates, release command changes, review trigger rules, and migration checklists.

## Goals

- Make the release path deterministic and fast.
- Keep one authoritative product line.
- Make `main` continuously releasable.
- Reduce release-day judgement and branch reconciliation.
- Make APS the authoritative ledger for intent and readiness.
- Prefer mechanical enforcement over policy memory.
- Support both human and agent execution without hidden state.
- Keep patch releases small, direct, and fast.

## Non-Goals

- Preserve GitFlow-style staging as a default.
- Require heavyweight council review on every PR.
- Treat release as an episodic all-day event.
- Move all release judgement into local-only scripts.
- Make APS a replacement for Git, CI, tags, or GitHub Releases.

## Required Artefact Set

The operating model should be implemented through small, explicit artefacts
rather than hidden convention. This section tracks the expected artefact family;
items can be added as the design is refined.

| Artefact | Purpose | Status |
| --- | --- | --- |
| Operating model spec | Normative Plan / Build / Release process | Proposed in this document |
| Migration plan | Stepwise move from `dev` integration to trunk-first `main` | Needed |
| Branching guide update | Human-facing branch rules after `dev` retirement | Needed |
| Worktree policy update | One permanent `main` worktree plus disposable branches | Needed |
| Release runbook update | Tag-from-main release process and recovery paths | Needed |
| Release skill update | Agent release orchestration against CI readiness and release records | Needed |
| APS schema/rules update | Machine-readable readiness, validation, and release-note metadata | Needed |
| Planning council playbooks | Creation-time and pre-execution plan validation workflows | Needed |
| Review trigger rules | Path/label matrix for pre-PR and PR-level reviews | Needed |
| Agent guidance script | Deterministic path-to-playbook/review/check guidance for hooks, agents, and CI | Needed |
| CI release-readiness workflow | Canonical pre-tag readiness gate keyed by SHA | Needed |
| Candidate artefact workflow | Non-publishing release artefact build before tag | Needed |
| Release record schema | Machine-readable link between tag, APS items, artefacts, and verification | Needed |
| Drift checks | APS/repo/release consistency checks | Needed |
| Rollback/incident playbook | Recovery flows for bad `main`, bad artefact, bad release | Needed |

The spec should stay concise enough to reason about under pressure. Detailed
procedure belongs in the downstream artefacts above.

## Cross-Stream Coherence Contract

This specification is the target-state authority for the Plan / Build / Release
operating model. The companion agentic and council specs define execution and
review mechanics that must implement this model, not alternate lifecycles.

Until migration completes, current-state documents may still describe `dev` as
the integration branch. Such documents are compatibility guidance only. Target
state artefacts must say so explicitly and must not silently mix `dev` promotion
with trunk-first `main` release semantics.

Normative boundaries:

| Concern | Canonical authority | Notes |
| --- | --- | --- |
| Intent, readiness, scope, dependencies | APS | APS is not a runtime log and does not embed CI output or agent transcripts. |
| Code history | Git | Work branches are disposable; target-state work branches branch from `main`. |
| Validation evidence | CI result for a commit SHA | Local checks are fast feedback, not release authority. |
| Released source | Annotated tag on `main` | Tags are immutable once external automation may have observed them. |
| Distributed artefacts | GitHub Release assets | cargo-dist logs are evidence, not the artefact authority. |
| Shipped-state reconciliation | Release record | The release record joins tag, APS items, artefacts, and verification. |
| Operator narrative and recovery log | GitHub tracking issue | The issue is durable operational context, not shipped-state truth. |
| Review judgement | Review/council session and PR summary | Review is evidence of critique, not validation proof. |

Shared lifecycle vocabulary:

```text
APS Draft -> APS Proposed -> APS Ready -> In Progress -> Merged -> Released/Shipped -> Complete/Archived
```

- `Ready` means execution is authorised.
- `In Progress` means work has started.
- `Merged` means code reached the integration target, but has not necessarily
  shipped.
- `Released` / `Shipped` means a release record proves inclusion in a verified
  release.
- `Complete` means the APS item or module has no remaining active closeout work
  and may be archived under APS rules.

`Committed` is treated as legacy wording for `Merged` unless a specific module
defines a narrower transition. New operating-model artefacts should prefer
`Merged` and `Released/Shipped`.

Review vocabulary is shared with the council spec:

| Term | Meaning |
| --- | --- |
| Targeted review | One selected reviewer role, usually pre-PR. |
| Mini council | Two selected reviewer roles for elevated risk. |
| Full council | Formal multi-reviewer review for system-changing work. |
| Planning council | Plan creation, direction validation, or pre-execution reality validation. |

Hook and agent vocabulary is shared with the agentic execution spec:

- Hooks are deterministic guardrails only.
- Skills route to playbooks and deterministic commands.
- Agents provide judgement, synthesis, and critique.
- Scripts and CI own deterministic execution.
- Sessions and events own continuity and recovery context.

## Operating Principles

- `main` is the only long-lived product branch.
- `main` must remain releasable after every merge.
- Normal work branches are short-lived and branch from `main`.
- `dev` is retired as a normal workflow branch after migration.
- `release/*` branches are exceptional, short-lived, and require explicit expiry.
- `hotfix/*` branches exist only for emergency repair paths.
- APS owns intent, scope, dependencies, acceptance criteria, validation, and
  release-note metadata.
- Git owns code history.
- CI owns validation evidence for commit SHAs.
- Tags own released source snapshots.
- GitHub Releases own distributed artefacts.
- Release records connect tags, artefacts, APS items, and verification evidence.

## Authoritative State Model

| Domain | Authority | Derived / Observed From |
| --- | --- | --- |
| Intent, scope, dependencies, acceptance criteria | APS | PR descriptions, branches, release notes |
| Code truth | `main` | Work branches and local worktrees |
| Review readiness | Pre-PR targeted review plus PR review state | Council output and comments |
| Validation truth | CI result for a commit SHA | Local preflight and logs |
| Release source snapshot | Annotated `v*` tag on `main` | GitHub Release metadata |
| Distributed artefacts | GitHub Release assets | cargo-dist logs and install smoke |
| User-facing release narrative | Generated notes from APS and merged PR metadata | Manual polish |
| Operational incidents | GitHub issue or APS ops item | Chat/session memory |

## State Machine

The normal state machine is:

```text
APS Draft
  -> APS Proposed
  -> APS Ready
  -> branch from main
  -> pre-PR targeted review
  -> PR Open
  -> PR Green
  -> Merged to main
  -> Main readiness green
  -> Included in release candidate
  -> Tagged
  -> Artefacts verified
  -> APS shipped state updated
```

No state transition should depend on unstated local context.

## Branching Specification

### Permanent Branches

`main` is the only permanent product branch.

`dev` must not be used for normal work after migration. If retained temporarily,
it must be treated as a compatibility branch with an expiry date and no new
normal PRs.

### Short-Lived Branches

Normal branches:

- `feat/*`
- `fix/*`
- `docs/*`
- `chore/*`

Rules:

- Branch from `main`.
- Target PRs to `main`.
- Keep branches small and disposable.
- Remove local worktrees and remote branches after merge.

### Exceptional Branches

`release/*` branches are allowed only when `main` cannot be tagged directly and
the release needs bounded stabilisation.

Requirements:

- Expiry must be explicit.
- Scope is limited to release hardening, packaging, docs, and final bug fixes.
- Lifetime target is hours to 48 hours.
- Existence of a `release/*` branch is a signal that normal continuous release
  flow failed or was intentionally paused.

`hotfix/*` branches are allowed only for urgent production repair.

Default hotfix path:

```text
branch from main -> fix -> targeted review -> PR to main -> CI -> merge -> patch tag
```

Emergency hotfix path when `main` is unreleasable:

```text
latest good tag -> hotfix/* -> minimal fix -> PR to main -> CI -> patch tag -> incident follow-up
```

## Planning Specification

Work starts from APS.

Planning should use council review at two gates:

1. **Plan creation or direction validation:** Planning Council reviews the
   proposed direction before an APS plan is treated as the execution source.
2. **Pre-execution reality validation:** Before work starts on a Ready plan or
   work item, Planning Council validates that the plan is still fit for purpose
   against current repository state.

This intentionally slows the start of substantial work. The trade-off is
accepted because early multi-perspective validation is cheaper than late
refactors caused by stale assumptions, missing dependencies, or repo drift.

Before implementation:

- Work item must be Ready or explicitly approved as urgent unplanned work.
- Acceptance criteria must be clear enough for review.
- Validation command must exist or the absence must be justified.
- User-visible changes should include release-note metadata.
- For non-trivial work, a Planning Council validation must be current enough to
  trust. If the repo, dependencies, branch base, or target architecture changed
  materially since the plan was written, rerun Planning Council validation before
  execution.

Recommended work item metadata:

```yaml
id: MOD-001
status: Ready
changeType: fix | feature | docs | internal | breaking
releaseIntent: candidate | hold | never
releaseScope: patch | minor | major | none
releaseNote:
  audience: user | operator | developer | none
  type: added | fixed | changed | removed | security
  text: optional one-sentence release note
validation:
  - command to prove the item
files:
  - path/or/glob
```

APS status must not be marked shipped from memory. Shipped state requires a
verified release record.

### Planning Council Gates

Planning Council has three modes in this operating model:

| Mode | When | Purpose | Output |
| --- | --- | --- | --- |
| Creation council | During new plan creation | Interrogate problem, negotiate direction, produce APS/ADR/spec artefacts | Proposed plan artefacts |
| Direction validation | After a plan is drafted or materially changed | Validate that the chosen direction is coherent before marking work Ready | Objections, amendments, or approval |
| Pre-execution validation | Immediately before executing a non-trivial Ready item or module | Check the plan against current repo reality and recent decisions | Proceed, amend, split, or replan recommendation |

Pre-execution validation must check:

- the target files still exist and have the expected shape
- relevant ADRs and specs have not changed the direction
- dependencies are still complete or correctly ordered
- validation commands still exist and are meaningful
- expected outcomes are still valuable
- work can still be sliced safely
- any as-built documentation requirements are known before coding starts

If Planning Council finds material drift, do not start implementation. Amend the
plan first, then restart the execution gate.

Small or urgent work may use a lightweight validation pass instead of a full
Planning Council, but the exception and reason should be visible in the PR or
APS item.

## Build Specification

Normal build flow:

```text
APS Ready -> branch from main -> implement smallest slice -> local checks -> targeted pre-PR review -> PR
```

Local checks are for fast feedback. They do not replace CI authority.

Recommended local check tiers:

- Targeted test or command for the changed area.
- Formatting for changed files.
- Lint/typecheck where relevant.
- Full local release preflight only when release readiness is being prepared.

## Review Specification

Review happens in tiers and at the earliest useful point.

```text
precommit = mechanical only
pre-PR = targeted agent review by default
PR = CI + human review + risk-triggered council
post-merge = drift and release-readiness checks
```

### Precommit

Precommit hooks must be fast, deterministic, and mechanical.

Allowed precommit work:

- format staged files
- lint staged files
- cheap static checks

Disallowed precommit work:

- council review
- slow multi-agent analysis
- release readiness judgement
- non-deterministic review steps

### Pre-PR Targeted Review

Every non-trivial agent-authored change should receive targeted agent review
before PR open.

Purpose:

- catch obvious defects before CI
- save reviewer attention
- let the authoring agent fix issues while context is hot
- reduce noisy PR threads

Default flow:

```text
local work
  -> targeted single-agent review
  -> fix findings
  -> open PR
```

### PR-Level Review

PR-level council is risk-triggered escalation, not the default first gate.

PR stage includes:

- CI
- human review where required
- structured council output when triggered
- APS and release metadata checks

### Review Tier Matrix

| Change Type | Review Mode | Timing |
| --- | --- | --- |
| Docs-only, typo, generated metadata | No council | PR CI only |
| Small local code change | Single targeted reviewer | Pre-PR |
| Medium feature/fix touching one subsystem | Single targeted reviewer; adversarial pass if risky | Pre-PR |
| Cross-boundary change | Mini council | Pre-PR or PR, based on risk |
| Security/auth/policy/release/CI changes | Mini or full council | Pre-PR targeted review, then PR escalation |
| Release candidate operating model or process change | Full council | PR, before adoption |
| Hotfix patch | Single targeted reviewer | Pre-PR; post-release review if rushed |
| Large architecture change | Planning council before implementation, full council before merge | Planning and PR |

### Reviewer Selection

| Risk Area | Reviewer |
| --- | --- |
| General correctness and maintainability | `Council — General` |
| Edge cases, failure paths, assumptions | `Council — Adversarial` |
| CI, release, deployment, operability | `Council — Operations` |
| Auth, secrets, policy, trust boundaries | `Council — Security` |
| Scope, proportionality, delivery risk | `Council — Pragmatic Lead` |

Recommended mini-council pairings:

| Change Shape | Reviewers |
| --- | --- |
| Feature behaviour | General + Adversarial |
| Release, CI, or process | Operations + Pragmatic Lead |
| Auth, policy, or security | Security + Adversarial |
| Architecture boundary | General + Pragmatic Lead |

### Mechanical Review Triggers

| Trigger | Required Review |
| --- | --- |
| `docs/**` only | none |
| `plans/**` only | Pragmatic Lead or General |
| `.github/**`, `scripts/release.sh`, `dist-workspace.toml` | Operations + Pragmatic Lead |
| `.claude/skills/release/**`, release runbooks | Operations + Adversarial |
| `crates/**` normal | General |
| `crates/**` auth/security/policy paths | Security + Adversarial |
| `packages/anvil/policy/**`, `policies/**` | Security + General |
| APS schema/rules | Pragmatic Lead + Operations |
| Branch/release/workflow model docs | Full council |
| Public installer/release artefact paths | Operations + Security |

Target rule:

```text
small PR = pre-PR single reviewer
risky PR = pre-PR single reviewer + PR mini council
system-changing PR = pre-PR targeted reviewer + PR full council
```

Patch exception:

```text
fix
  -> targeted pre-PR reviewer
  -> PR
  -> CI
  -> merge
  -> release
```

If PR council is skipped for speed during an urgent patch, open a post-release
review item and run mini or full council within 24 hours.

## PR Specification

All normal PRs target `main`.

PRs should include:

- APS work item ID or explicit unplanned-work reason.
- Summary of behavioural change.
- Validation evidence.
- Release-note metadata if user-visible.
- Council or targeted-review output when required.

PRs must not be used as `dev -> main` promotion containers. There is no normal
promotion branch in the target model.

## CI Specification

CI should use risk and changed paths, not branch tiering, as the primary
selector.

| Tier | Trigger | Purpose | Gate |
| --- | --- | --- | --- |
| Fast PR | every PR | formatting, lint, typecheck, affected tests | required |
| Full PR | risky paths or label | full Rust + TS tests, policy tests, relevant e2e | required when selected |
| Main post-merge | every merge to `main` | full validation and release readiness status | required for release |
| Candidate | manual or scheduled on green `main` | cargo-dist plan, smoke, generated notes | required before tag |
| Tag release | `v*` tag | build and publish immutable artefacts | release gate |
| Post-publish | after release workflow | verify assets, installer, latest pointers | release closeout |

Local preflight scripts remain useful, but canonical release readiness must be
recorded by CI against a commit SHA.

## Release Specification

Release is publishing a verified `main` snapshot, not assembling branch state.

Default release flow:

```text
1. Select current green main SHA.
2. Generate release candidate metadata from APS and merged PRs since previous tag.
3. Compute proposed version from change metadata.
4. Run release-readiness workflow for the SHA.
5. Build candidate artefacts without publishing when required.
6. Open a release-prep PR only if generated files must change.
7. Merge release-prep PR to main.
8. Tag the exact green main SHA.
9. Release workflow builds and publishes artefacts.
10. Post-publish verification confirms artefacts, installers, and latest pointers.
11. Release record is emitted.
12. APS shipped state updates from the release record.
```

Fast patch release flow:

```text
1. Merge fix PR to main.
2. Run release-readiness workflow for patch scope.
3. Generate patch notes from APS and PR metadata.
4. Tag the green main SHA.
5. Publish artefacts.
6. Verify install and release assets.
7. Update APS shipped state from the release record.
```

There is no normal `dev -> main` release PR and no normal back-merge PR.

## Versioning Specification

Version should be computed from release candidate metadata:

- `fix` changes imply patch.
- `feature` changes imply minor.
- `breaking` changes imply major.
- `docs` and `internal` changes do not require a release unless bundled with
  releasable work.
- Beta suffix conventions are applied consistently by release tooling.

Manual override is allowed, but the override and reason must be recorded in the
release candidate output.

## Release Notes And Changelog Specification

Release notes should be generated from APS and merged PR metadata, then manually
polished if needed.

Required metadata for user-visible changes:

- audience
- type
- one-sentence text
- optional docs link

`CHANGELOG.md` may remain as a human-facing artefact, but should not be the only
source used to reconstruct release contents.

## Artefact Specification

Tag publish must build immutable artefacts from the tagged source snapshot.

Required release verification:

- expected platform archives exist
- installers exist
- checksums or equivalent integrity metadata exist
- private and public release records are consistent
- install command reaches the expected public artefact
- `/releases/latest` points to the intended release where applicable

Release workflow should emit a machine-readable release record:

```json
{
  "version": "vX.Y.Z",
  "sourceSha": "...",
  "previousTag": "vX.Y.Z",
  "apsItems": ["MOD-001"],
  "assets": [{ "name": "...", "sha256": "..." }],
  "privateRelease": "...",
  "publicRelease": "...",
  "verifiedAt": "..."
}
```

## Drift Control Specification

### APS vs Repo Drift

Checks should detect:

- changed tracked files without APS references
- APS item status inconsistent with PR or merge state
- module counts inconsistent with module files
- Complete items without validation evidence
- generated release candidate contents missing merged APS items

Start these as warnings, then promote stable checks to required gates.

### Repo vs Release Drift

Checks should detect:

- version files disagreeing with tag
- GitHub Release missing expected artefacts
- public and private releases diverging unexpectedly
- install latest resolving to the wrong version
- APS shipped state without matching release record

## Recovery Specification

### Bad Commit On Main Before Release

Default recovery:

```text
revert on main -> CI green -> continue
```

### Bad Release Before Public Use

Default recovery:

```text
stop distribution if safe -> fix on main -> new patch tag
```

Do not reuse version tags once external automation may have observed them.

### Bad Release After Public Use

Default recovery:

```text
incident issue -> fix or revert on main -> patch tag -> mark previous release superseded
```

### Main Is Unreleasable

Treat unreleasable `main` as an incident.

Permitted responses:

- revert the offending commit
- fix forward immediately
- branch from latest good tag only if an urgent patch cannot wait for `main`
  repair

## Agent Workflow Specification

Agents should operate through explicit commands or skills:

- `aps next`: select Ready work and create a branch from `main`.
- `aps reconcile`: compare changed files, commits, PRs, and APS status.
- `plan validate`: run Planning Council reality validation before executing a
  non-trivial Ready item.
- `build ready`: run local fast checks and request CI readiness if needed.
- `review pre-pr`: run targeted single-agent review before PR open.
- `release candidate`: compute version, notes, APS items, and validations for a
  selected `main` SHA.
- `release publish`: tag an already-ready SHA and monitor publication.
- `release verify`: verify artefacts and update APS shipped state.

Agent invariants:

- Never decide branch authority from memory.
- Never infer release contents from prose alone.
- Never mark shipped without tag, CI, artefact, and release-record evidence.
- Prefer one explicit state transition per command.

## Human Workflow Specification

Normal human flow:

```text
1. Pick APS item.
2. Create branch from main.
3. Implement and run local checks.
4. Run or request targeted pre-PR review if non-trivial.
5. Open PR to main.
6. Merge when CI and required review are green.
7. Release whenever useful from green main.
```

Patch human flow:

```text
1. Fix bug on branch from main.
2. Run targeted pre-PR review.
3. Open PR to main.
4. Merge when CI is green.
5. Run release readiness for patch scope.
6. Tag green main SHA.
7. Verify release assets and install smoke.
```

## Migration Specification

Migration should be staged:

1. Add observability for `main`/`dev` divergence and APS drift.
2. Promote current `dev` to `main` using the existing runbook.
3. Stop accepting normal new PRs into `dev`.
4. Retarget normal work to `main`.
5. Move CI gates to main-first operation.
6. Add release-readiness workflow keyed by SHA.
7. Add candidate artefact build without publishing.
8. Add APS metadata and drift checks in warning mode.
9. Add Planning Council validation gates for plan creation and pre-execution.
10. Add deterministic agent guidance for hooks, agents, and CI.
11. Retire or protect `dev` against normal pushes.
12. Remove `dev -> main`, normal `release/*`, and back-merge steps from runbooks
    and agent skills.

## Minimum Viable Operating Model

The smallest acceptable version of this specification is:

```text
APS -> branch from main -> pre-PR targeted review -> PR to main -> CI -> merge -> tag -> release -> verify -> APS shipped
```

Minimum rules:

- one permanent branch: `main`
- every task starts from APS or explicit urgent exception
- every normal PR targets `main`
- every merge leaves `main` releasable
- release means tagging a green `main` SHA
- CI builds and publishes from tags
- APS shipped state updates only from verified release records

## Open Implementation Decisions

- Exact APS metadata schema and parser location.
- Whether release candidate output is committed, uploaded as CI artefact, or both.
- Whether generated changelog updates happen before tag or as part of candidate
  PR.
- Exact path labels and required-review automation.
- Exact definition of candidate artefact build frequency.
- Whether `dev` is deleted, archived, or protected after migration.
- Exact freshness rule for Planning Council pre-execution validation.
- Exact location and output schema for deterministic agent guidance.
