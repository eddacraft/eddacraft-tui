# Plan / Build / Release Operating Model Rebaseline

Date: 2026-05-09

Scope: Plan, Build, Release

Prior context:

- `.claude/skills/release/SKILL.md`
- `docs/guides/release-runbook.md`
- `docs/guides/branching-strategy.md`
- `docs/guides/worktree-policy.md`
- `docs/runbooks/branch-reconciliation.md`
- `plans/index.aps.md`
- `RELEASE-PLAN.md`

This review intentionally does not optimise the current release process in
place. The current process has already been improved: deterministic preflight is
separated from judgement, the release skill is resumable from live state, and
tracking issues are used as durable logs. Those are real improvements. They do
not solve the larger system problem: Plan, Build, and Release still have too
many authorities and too many branch-state transitions for an AI-native team.

## Executive Recommendation

Move to a trunk-first operating model:

- `main` is the only long-lived product branch.
- `dev` is retired after migration, or downgraded to a temporary compatibility
  branch with an expiry date.
- Every merged PR to `main` must leave `main` releasable.
- APS becomes the authoritative intent and readiness ledger.
- GitHub commits, PRs, CI, tags, releases, and artefacts become derived
  execution records, not competing sources of truth.
- Release becomes a continuous capability: create a candidate from the current
  green `main`, publish from immutable tags, and patch from `main` unless a
  production-only emergency requires a short-lived hotfix branch.

The fastest safe path is:

```text
APS Ready item -> short branch -> PR -> CI + review -> merge to main -> optional tag -> release workflow -> artefact verification -> APS shipped state
```

The central simplification is removing `dev -> main` promotion as a release
event. Promotion is where drift, ambiguity, review duplication, and release-day
cognitive load accumulate.

## Current-State Assessment

### Branching

Current state:

- `main` is described as the stable release branch.
- `dev` is described as the active integration branch.
- Normal work branches target `dev`.
- Releases promote `dev -> main` directly, or through `release/*`.
- Hotfixes branch from `main` and must be merged back to `dev`.

Assessment:

- The model is coherent on paper but fragile in operation.
- It requires humans and agents to remember branch directionality under
  pressure.
- It creates two long-lived product truths: the code people are building on and
  the code users receive.
- It makes releases a reconciliation problem rather than a packaging problem.
- It has already produced branch divergence severe enough to require a recovery
  runbook.

### PR Strategy

Current state:

- Feature PRs target `dev`.
- Release PRs target `main`.
- Release hardening may happen on `release/*`.
- Back-merge PRs are required after release-only fixes.

Assessment:

- PRs are doing two jobs: reviewing change quality and moving state between
  branches.
- The `dev -> main` PR is high ceremony and low signal when it re-reviews work
  already merged to `dev`.
- The back-merge PR is pure drift repair. Its existence is evidence that branch
  topology is manufacturing work.

### Release Sequencing

Current state:

- Run `./scripts/release.sh` locally.
- Invoke `/release`.
- Assess live state.
- Open a release tracking issue.
- Bump versions and changelog.
- Promote branch state to `main`.
- Tag `main`.
- Monitor cargo-dist.
- Verify artefacts and docs.
- Back-merge if needed.

Assessment:

- The process is safer than before, but still too episodic.
- Patch releases can take a day because release work includes branch promotion,
  version edits, changelog synthesis, workflow monitoring, and state repair.
- The release skill is forced to compensate for state fragmentation rather than
  orchestrating a small deterministic publish operation.

### Versioning

Current state:

- Versions are lockstep for release surfaces.
- Version bump happens as part of release prep.
- Beta suffix conventions are manually selected.
- `CHANGELOG.md`, public docs, package manifests, Cargo workspace version, tags,
  GitHub releases, and install latest state all need alignment.

Assessment:

- Version state is duplicated across too many files and systems.
- Manual version bumping is high-risk because it happens late in the process.
- Patch speed is constrained by edit surfaces that could be generated or
  checked.

### Source-of-Truth Ownership

Current competing sources of truth:

- APS index and module files for work intent and status.
- `RELEASE-PLAN.md` for release slate and candidate status.
- Branch state for what is integrated.
- PR state for review and merge status.
- CI state for validation.
- Tags for released source snapshots.
- GitHub Releases for published artefacts.
- Changelog and public docs for user-facing release facts.
- Release tracking issues for operational logs.
- Local agent session state for release progress.

Assessment:

- This is the dominant root cause. The workflow does not have one state machine.
- Each tool owns part of reality, and agents must reconstruct reality from live
  queries each turn.
- Resumability is achieved by re-reading fragmented state, not by reducing the
  number of state holders.

### APS Integration

Current state:

- APS is mandatory and rich, but mostly documentary.
- Commits and PRs may include APS references.
- Plans describe release slates and work items.
- Plan state can drift from repo state and release state.

Assessment:

- APS is the right place for intent, dependencies, readiness, and acceptance
  criteria.
- APS is not yet executable enough to drive CI selection, release eligibility,
  or changelog generation.
- Status transitions are too manual for an agent-heavy workflow.

### CI/CD Boundaries

Current state:

- CI runs on `main` and `dev` pushes and PRs.
- Rust workflow has additional path filters and cross-compile gates.
- cargo-dist PR mode is `plan` only; full artefacts build on tags.
- Local release preflight duplicates important CI checks.

Assessment:

- CI has useful tiers, but branch-based behaviour encodes the old topology.
- Full release artefact validation is too late if it only happens after a tag.
- Local preflight is valuable for fast feedback, but should not be a hidden gate
  that only exists on the operator's machine.

## Root-Cause Analysis

### 1. The Workflow Has Two Product Lines

`dev` is where product reality evolves. `main` is where users receive releases.
That split forces every release to answer: which branch is truth today?

The branch reconciliation runbook shows the failure mode clearly: `main` became
a release integration line while `dev` kept structural truth. The process then
had to recover behaviour instead of simply shipping a known-good commit.

### 2. Release Is Treated As State Assembly

Release should package a validated source snapshot. In the current model it also
selects branch strategy, performs version edits, opens tracking issues, promotes
branches, monitors workflows, verifies public/private releases, and repairs
back-merge drift.

This is too much cognition for patch work.

### 3. State Transitions Are Policy, Not Mechanics

Rules such as "merge `main` fixes back to `dev` the same day" are correct but
weak. They rely on behaviour, not enforcement. Humans and agents can forget.

The target model should make invalid states impossible or visibly failing:

- No hidden release-only commits.
- No branch promotion diff large enough to review late.
- No release tag if generated version files disagree.
- No APS item marked shippable if required validation is absent.

### 4. APS Is Mandatory But Not Authoritative Enough

APS describes work, but CI and release do not consume it as a first-class input.
As a result, APS can be correct while branch state is wrong, or branch state can
be correct while APS lags.

### 5. Agents Need Different Workflow Primitives

Human workflows tolerate implicit context and social memory. Agent workflows
need machine-readable state, deterministic entrypoints, and resumable commands.

Current agent skills spend too much effort reconstructing reality because the
system has not exposed a single operational state model.

## Proposed Target-State Operating Model

### Principles

- Main is permanently releasable.
- Plans define intent; Git records execution; CI records validation; tags record
  released snapshots; releases record distributed artefacts.
- There is no long-lived integration branch.
- Every workflow transition is either mechanically enforced or mechanically
  checked.
- Release is incremental by default and heavyweight only by exception.
- Agents operate through commands that read and write explicit state, not prose
  runbooks alone.

### Authoritative Ownership Model

| Domain                                           | Authority                                                       | Derived / Observed From                            |
| ------------------------------------------------ | --------------------------------------------------------------- | -------------------------------------------------- |
| Intent, scope, dependencies, acceptance criteria | APS                                                             | PR descriptions, branch names, changelog fragments |
| Code truth                                       | `main`                                                          | Work branches, local worktrees                     |
| Review decision                                  | PR checks and review state                                      | Council output, comments                           |
| Validation truth                                 | CI check suites on commit SHA                                   | Local preflight, logs                              |
| Release source snapshot                          | Annotated `v*` tag on `main`                                    | GitHub Release, installers                         |
| Distributed artefacts                            | GitHub Release assets                                           | cargo-dist logs, install site                      |
| User-facing change narrative                     | Generated changelog/release notes from APS + merged PR metadata | Manual edits                                       |
| Operational incidents and exceptions             | GitHub issue or APS ops item                                    | Chat/session memory                                |

### Target Workflow Diagram

```text
                  ┌────────────────────┐
                  │ APS module / item  │
                  │ Ready + validated  │
                  └─────────┬──────────┘
                            │ create work branch
                            ▼
                  ┌────────────────────┐
                  │ feat/fix/docs/*    │
                  │ disposable branch  │
                  └─────────┬──────────┘
                            │ PR to main
                            ▼
       ┌────────────────────────────────────────────┐
       │ PR: APS references + CI + council as needed │
       └────────────────────┬───────────────────────┘
                            │ merge only if green
                            ▼
                  ┌────────────────────┐
                  │ main               │
                  │ always releasable  │
                  └─────────┬──────────┘
                            │ optional release intent
                            ▼
                  ┌────────────────────┐
                  │ vX.Y.Z tag         │
                  │ immutable source   │
                  └─────────┬──────────┘
                            │ release workflow
                            ▼
                  ┌────────────────────┐
                  │ artefacts + latest │
                  │ verified publish   │
                  └────────────────────┘
```

### Target State Machine

```text
APS Draft
  -> APS Proposed
  -> APS Ready
  -> PR Open
  -> PR Green
  -> Merged to main
  -> Included in release candidate
  -> Tagged
  -> Artefacts verified
  -> Shipped
  -> Archived / Complete
```

The state machine can be stored in APS with derived fields checked by
automation. Manual status edits remain possible, but CI should detect impossible
states.

## Recommended Branch / Release Lifecycle

### Branches

Recommended steady state:

| Branch                                 | Purpose                                             | Lifetime          |
| -------------------------------------- | --------------------------------------------------- | ----------------- |
| `main`                                 | Only product integration and release branch         | Permanent         |
| `feat/*`, `fix/*`, `docs/*`, `chore/*` | Work branches from `main`                           | Hours to days     |
| `hotfix/*`                             | Emergency branch from latest released tag or `main` | Hours             |
| `release/*`                            | Exceptional stabilisation only, requires expiry     | Hours to 48 hours |
| `dev`                                  | Retired                                             | None              |

Default rule:

```text
branch from main -> PR to main -> merge -> release from main
```

Exceptional hotfix rule:

```text
latest release tag -> hotfix/* -> PR to main -> tag patch from merged main
```

Only branch from a tag directly if `main` contains unreleasable work. In the
target model, that should be treated as an incident because `main` is supposed
to be permanently releasable.

### PR Strategy

- All normal PRs target `main`.
- PRs must reference APS item IDs unless explicitly marked as unplanned
  operational work.
- Required CI is based on changed files, not target branch differences.
- Every non-trivial agent-authored change should receive targeted agent review
  before PR open.
- PR-level council is an escalation path for risky or system-changing changes,
  not the default first review gate.
- Any PR-level council result should attach to the PR as structured output.
- PR size limits should be enforced by guidance and optionally by a warning bot,
  not by release branches.

### Review Timing

Review should happen as early as it can provide useful feedback, but not so
early that it slows local iteration. The target split is:

```text
precommit = mechanical only
pre-PR = targeted agent review by default
PR = CI + human review + risk-triggered council
post-merge = drift and release-readiness checks
```

Do not run council as a literal Git pre-commit hook. Pre-commit hooks should be
fast, deterministic, and mechanical. Agent review belongs before PR open, once
the change is coherent enough to critique.

| Stage      | Purpose                                   | Should Run                                                      |
| ---------- | ----------------------------------------- | --------------------------------------------------------------- |
| Precommit  | Stop obviously broken staged changes      | format, lint-staged, cheap static checks                        |
| Pre-PR     | Improve quality before externalising work | targeted single-agent review; mini council for medium-risk work |
| PR         | Validate merge readiness                  | CI, human review, required risk-triggered council               |
| Post-merge | Detect drift and release readiness issues | APS reconciliation, release readiness, candidate checks         |

Default flow:

```text
local work
  -> targeted single-agent review
  -> fix findings
  -> open PR
  -> CI + human review
  -> optional PR council if triggered
  -> merge
```

This saves CI cycles and reviewer attention by catching issues while the agent
or human still has fresh local context.

### Review Tiers

Use review tiers by risk, not habit.

| Change Type                                        | Review Mode                                                       | Timing                                     |
| -------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------ |
| Docs-only, typo, generated metadata                | No council                                                        | PR CI only                                 |
| Small local code change                            | Single targeted reviewer                                          | Pre-PR                                     |
| Medium feature/fix touching one subsystem          | Single targeted reviewer; adversarial pass if risky               | Pre-PR                                     |
| Cross-boundary change                              | Mini council                                                      | Pre-PR or PR, based on risk                |
| Security/auth/policy/release/CI changes            | Mini or full council                                              | Pre-PR targeted review, then PR escalation |
| Release candidate operating model / process change | Full council                                                      | PR, before adoption                        |
| Hotfix patch                                       | Single targeted reviewer                                          | Pre-PR; post-release review if rushed      |
| Large architecture change                          | Planning council before implementation, full council before merge | Planning and PR                            |

Recommended reviewer selection:

| Risk Area                               | Reviewer                   |
| --------------------------------------- | -------------------------- |
| General correctness and maintainability | `Council — General`        |
| Edge cases, failure paths, assumptions  | `Council — Adversarial`    |
| CI, release, deployment, operability    | `Council — Operations`     |
| Auth, secrets, policy, trust boundaries | `Council — Security`       |
| Scope, proportionality, delivery risk   | `Council — Pragmatic Lead` |

Recommended pairings for mini council:

| Change Shape              | Reviewers                   |
| ------------------------- | --------------------------- |
| Feature behaviour         | General + Adversarial       |
| Release, CI, or process   | Operations + Pragmatic Lead |
| Auth, policy, or security | Security + Adversarial      |
| Architecture boundary     | General + Pragmatic Lead    |

Mechanical trigger rules should be encoded as labels or path-based automation:

| Trigger                                                   | Required Review             |
| --------------------------------------------------------- | --------------------------- |
| `docs/**` only                                            | none                        |
| `plans/**` only                                           | Pragmatic Lead or General   |
| `.github/**`, `scripts/release.sh`, `dist-workspace.toml` | Operations + Pragmatic Lead |
| `.claude/skills/release/**`, release runbooks             | Operations + Adversarial    |
| `crates/**` normal                                        | General                     |
| `crates/**` auth/security/policy paths                    | Security + Adversarial      |
| `packages/anvil/policy/**`, `policies/**`                 | Security + General          |
| APS schema/rules                                          | Pragmatic Lead + Operations |
| Branch/release/workflow model docs                        | Full council                |
| Public installer/release artefact paths                   | Operations + Security       |

Target rule:

```text
small PR = pre-PR single reviewer
risky PR = pre-PR single reviewer + PR mini council
system-changing PR = pre-PR targeted reviewer + PR full council
```

Patch-release exception:

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

### Release Lifecycle

Default release:

```text
1. Select current green main SHA.
2. Generate release candidate metadata from APS + merged PRs since previous tag.
3. Compute version.
4. Open or update a release-prep PR only if generated files must change.
5. Merge release-prep PR to main.
6. Tag the exact main SHA.
7. CI builds artefacts from the tag.
8. CI verifies required assets on private and public releases.
9. APS items included in the tag move to Shipped.
```

Fast patch release:

```text
1. Merge fix PR to main.
2. Run release candidate check for patch scope.
3. Tag patch version from main.
4. Let release workflow publish.
5. Verify assets and install command.
```

No `dev -> main` PR. No back-merge PR. No release branch unless `main` is
broken, which should be treated as an operational incident.

## APS Evolution Recommendations

APS should remain mandatory, but evolve from planning prose into an executable
intent ledger.

### Add Machine-Readable Front Matter

Each module should expose stable metadata:

```yaml
aps:
  module: LAUNCH
  status: In Progress
  releaseIntent: candidate | hold | never
  releaseScope: patch | minor | major | none
  owner: agent-or-human
  validation:
    - pnpm test
    - cargo test --workspace
```

Each work item should expose:

```yaml
id: LAUNCH-014
status: Ready
files:
  - crates/anvil-cli/src/...
validation:
  - cargo test -p eddacraft-anvil
releaseNote: user-visible sentence or null
changeType: fix | feature | docs | internal | breaking
```

Markdown can remain the human interface, but automation needs a parseable block.

### Define APS-Derived Release Sets

Replace `RELEASE-PLAN.md` as a competing release-slate source with generated
release views:

- Candidate items: APS items with `releaseIntent: candidate` and merged commits.
- Held items: APS items marked `hold` with a reason.
- Shipped items: APS items included in a tag.
- Drift items: APS items whose files changed but status did not move.

`RELEASE-PLAN.md` can become a generated or mostly generated view. It should not
be independently edited as the canonical release slate.

### Add Mechanical Drift Checks

CI should fail or warn when:

- A PR modifies tracked files but lacks APS references.
- A PR references APS items that are not Ready or In Progress.
- APS item status says Complete but validation has not passed on the merge SHA.
- `plans/index.aps.md` module counts disagree with module files.
- Release notes omit user-visible merged APS items.

Start with warnings, then promote stable checks to required status.

## CI/CD Architecture Recommendations

### CI Tiers

Use commit-risk tiers instead of branch tiers.

| Tier            | Trigger                             | Purpose                                                | Gate                   |
| --------------- | ----------------------------------- | ------------------------------------------------------ | ---------------------- |
| Fast PR         | every PR                            | formatting, lint, typecheck, affected tests            | required               |
| Full PR         | risky paths or label                | full Rust + TS tests, policy tests, e2e where relevant | required when selected |
| Main post-merge | every merge to `main`               | full repo validation and release readiness status      | required for release   |
| Candidate       | manual or scheduled on green `main` | cargo-dist plan, package smoke, changelog generation   | required before tag    |
| Tag release     | `v*` tag                            | build and publish immutable artefacts                  | release gate           |
| Post-publish    | after release workflow              | verify assets, install command, latest pointers        | release closeout       |

### Replace Local-Only Release Preflight With CI-Backed Readiness

Keep `./scripts/release.sh` for fast local confidence, but make the canonical
release gate a CI readiness workflow keyed by SHA:

```text
gh workflow run release-readiness.yml -f sha=<main-sha> -f scope=patch
```

`release-readiness.yml` is a proposed workflow to be added during the migration,
not a command that exists today. The release skill should read that workflow
result instead of asking whether a local script passed once the workflow exists.

### Build Artefacts Earlier Without Publishing

cargo-dist `pr-run-mode = "plan"` keeps PRs cheap, but it means platform build
failures appear late. Add a non-publishing candidate workflow for release
readiness:

- Builds all target artefacts for the selected SHA.
- Uploads them as workflow artefacts.
- Does not create GitHub Releases.
- Runs on demand and nightly when `main` changed.

This preserves cheap PRs while preventing tag-time surprises.

### Make Release Workflow Stateful in GitHub, Not Locally

The release workflow should emit a machine-readable release record:

```json
{
  "version": "v0.6.1-beta",
  "sourceSha": "...",
  "previousTag": "v0.6.0-beta",
  "apsItems": ["V060F-002"],
  "assets": [{ "name": "...", "sha256": "..." }],
  "privateRelease": "...",
  "publicRelease": "...",
  "verifiedAt": "..."
}
```

Store it as a release asset and optionally commit a generated pointer under
`plans/releases/`. The tag and GitHub Release remain authoritative for shipped
source and artefacts; APS consumes the record to update shipped state.

## Validation Layers

Validation should be layered by cost and reversibility.

| Layer           | Runs At                 | Validates                                       |
| --------------- | ----------------------- | ----------------------------------------------- |
| Save/local      | developer or agent loop | formatting, targeted tests, obvious regressions |
| PR fast         | PR                      | affected correctness and style                  |
| PR deep         | risky PRs               | broader integration and policy behaviour        |
| Main readiness  | post-merge              | permanent releasability                         |
| Candidate build | selected SHA            | release packaging and platform artefacts        |
| Tag publish     | tag                     | immutable release build                         |
| Post-publish    | release complete        | assets, installers, latest, smoke commands      |

Do not rely on one heavyweight gate. Fast feedback should catch most issues;
expensive validation should be close to release but not after irreversible
publication.

## Agent Workflows

Agents need commands, not just policies.

Recommended commands or skills:

- `aps next`: select the next Ready item and create a branch from `main`.
- `aps reconcile`: compare changed files, commits, PRs, and APS status.
- `build ready`: run local fast checks and request CI readiness if needed.
- `release candidate`: compute version, release notes, APS items, and required
  validations for a selected `main` SHA.
- `release publish`: tag an already-ready SHA and monitor publication.
- `release verify`: verify assets and update APS shipped state.

Agent invariants:

- Agents should never decide branch authority from memory.
- Agents should never infer release contents from prose alone.
- Agents should never update APS shipped state without a tag or release record.
- Agents should prefer one explicit state transition per command.

## Human Workflows

Humans should experience the model as simpler, not more automated ceremony.

Normal human flow:

```text
1. Pick APS item.
2. Create branch from main.
3. Open PR to main.
4. Merge when green.
5. Release whenever useful.
```

Patch human flow:

```text
1. Fix bug on branch from main.
2. Merge to main.
3. Run release candidate for patch.
4. Approve tag.
5. Confirm install smoke.
```

The human should not need to reason about whether a change is on `dev`, `main`,
a release branch, and a tracking issue simultaneously.

## Local Development Ergonomics

Recommended worktree policy:

- Keep one permanent worktree for `main`.
- Use disposable worktrees for active branches.
- Do not keep a permanent `dev` worktree after migration.
- Worktree name should include APS item or branch slug.
- Local scripts should accept `--changed`, `--all`, and `--release-readiness`
  modes so agents can choose the cheapest valid check.

Recommended local command split:

- `pnpm verify:fast`: proposed alias for format, lint, affected typecheck/tests.
- `pnpm verify:full`: proposed alias for full TS checks.
- `cargo xtask verify`: proposed Rust verification wrapper, or equivalent.
- `./scripts/release.sh`: retained as local release readiness mirror, but not
  the authoritative gate.

Current repo commands remain `pnpm format:check`, `pnpm lint:check`,
`pnpm typecheck`, `pnpm test`, and `pnpm run lint:rust` until those aliases or
wrappers are implemented.

## Recovery / Rollback Flows

### Bad Commit On Main Before Release

Preferred recovery:

```text
revert commit on main -> CI green -> continue
```

Do not create a `dev` repair branch.

### Bad Release Artefact Before Public Use

Preferred recovery:

```text
delete bad release artefacts if safe -> fix -> new patch tag
```

Avoid reusing version tags once public automation may have observed them.

### Bad Release After Public Use

Preferred recovery:

```text
open incident issue -> revert or fix on main -> patch tag -> mark previous release as superseded
```

Do not mutate history. Do not force-push tags. Publish a new patch.

### Main Is Not Releasable

This is an incident in the target model.

Immediate options:

- Revert the breaking commit on `main`.
- If a patch is urgent and revert is unsafe, branch `hotfix/*` from the last
  good tag, apply the minimal fix, PR back to `main`, then tag.
- Open an APS operational item capturing why `main` became unreleasable.

## Release Artefact Generation

Keep cargo-dist as the artefact builder unless it blocks required automation.
The target model changes when artefacts are built, not necessarily the builder.

Recommendations:

- Add non-publishing candidate builds from selected `main` SHAs.
- Generate SHA256 checksums and include them in release records.
- Verify expected asset names mechanically in CI.
- Verify private and public releases in a post-publish job, not only by the
  release agent.
- Keep public/private release duality if distribution needs it, but treat the
  public release as the distribution record and the tag as the source record.

## Changelog / Release Note Generation

Current changelog work is too manual and too late.

Recommended model:

- PRs carry structured change metadata from APS: `changeType`, `releaseNote`,
  `breaking`, `migration`, `operatorNote`.
- Release candidate generation groups merged PRs since the previous tag.
- `CHANGELOG.md` is generated or semi-generated from those records.
- Manual editing is allowed only as a polishing pass, not as the primary source.
- Public docs version snippets should be generated from the selected release
  version.

Minimum useful release note schema:

```yaml
releaseNote:
  audience: user | operator | developer | none
  type: added | fixed | changed | removed | security
  text: 'One sentence in user-facing language.'
  docs: optional/path.md
```

## Drift Controls

### Plan State vs Repo State

Failure mode:

- APS says work is Ready or In Progress, but code has merged.
- APS says Complete, but validation did not run.
- APS counts disagree with module files.

Controls:

- CI job validates APS syntax and counts.
- PR check maps changed files to APS items.
- Merge commits or PR metadata include APS IDs.
- `aps reconcile` proposes status changes after merge.
- Completion requires validation evidence on a commit SHA.

### Repo State vs Release State

Failure mode:

- Version files, tag, GitHub Release, install latest, changelog, and public docs
  disagree.

Controls:

- Release candidate job computes all expected version surfaces before tag.
- Tag workflow writes a release record.
- Post-publish job verifies artefacts and latest pointers.
- Generated docs snippets read from the release record or package version.
- APS shipped state only updates from release record, not manual memory.

## Explicit Tradeoffs

### Retiring `dev`

Benefits:

- Removes the largest drift vector.
- Removes release promotion PRs.
- Removes back-merge repair.
- Makes patch releases much faster.
- Gives agents one branch authority.

Costs:

- Requires stronger PR gates on `main`.
- Requires feature flags or smaller slices for incomplete work.
- Reduces the psychological buffer of an integration branch.
- Forces discipline: `main` cannot be a dumping ground.

Recommendation: accept the costs. They are healthier than branch drift.

### Continuous Releases vs Heavy Release Events

Benefits:

- Smaller diffs per release.
- Easier rollback and patching.
- Less release-day cognition.
- Better fit for agent execution.

Costs:

- Requires better generated notes.
- Requires release readiness to be cheap and repeatable.
- May produce more release records.

Recommendation: make releases cheap enough to be routine. Keep themed release
communication as a marketing layer, not as a branch topology.

### APS As Executable Source

Benefits:

- Reduces plan/repo/release drift.
- Gives agents deterministic work selection.
- Enables generated changelogs and release candidates.

Costs:

- APS format must become stricter.
- Markdown-only ergonomics may suffer unless tooling helps.

Recommendation: evolve APS with parseable blocks, not a wholesale replacement.

### CI As Stateful Orchestrator

Benefits:

- Release readiness is tied to SHA, not local operator memory.
- Resumability becomes native to GitHub workflows.
- Agents can query one durable run.

Costs:

- More workflow code.
- More GitHub Actions minutes.

Recommendation: move canonical release readiness into CI. Keep local scripts as
mirrors.

## Migration Strategy

### Phase 0: Freeze The Current Rules In Place

Do not immediately delete `dev`. First make the current system observable.

- Add a CI check that reports `main..dev` and `dev..main` divergence.
- Add an APS drift check in warning mode.
- Add release candidate dry-run workflow for current `dev -> main` model.
- Document that `release/*` requires an explicit expiry.

### Phase 1: Make Main Releasable After Every Promotion

- Promote current `dev` to `main` using the existing runbook.
- Immediately stop accepting new normal work into `dev`.
- Require new PRs to target `main` unless explicitly exempted.
- Keep `dev` as a compatibility branch for open work only.

### Phase 2: Move CI Gates To Main-First

- Change branch filters so PRs to `main` receive the normal fast and deep gates.
- Keep path-based cost controls.
- Add release-readiness workflow keyed by `main` SHA.
- Add candidate artefact build without publishing.

### Phase 3: Make APS Executable Enough

- Add machine-readable work item metadata.
- Add APS validation and count checks to CI.
- Add PR-to-APS reference checks in warning mode.
- Generate release candidate notes from APS + merged PRs.

### Phase 4: Retire Dev

- Close or retarget remaining PRs from `dev` to `main`.
- Archive `dev` branch or protect it against pushes.
- Update branching, worktree, release, and agent docs.
- Remove back-merge steps from release skill and runbook.

### Phase 5: Tighten Enforcement

- Promote APS drift checks from warning to required where stable.
- Require release readiness workflow before tag.
- Require post-publish verification before release closeout.
- Generate changelog and version doc snippets.

## Likely Failure Modes In The Proposed Design

### Main Becomes Unreleasable

Cause:

- Large incomplete work merges without flags or slice discipline.

Mitigation:

- Feature flags, smaller PRs, revert-first culture, main readiness checks.

### CI Cost Increases Too Much

Cause:

- All PRs target `main`, so teams over-trigger expensive validation.

Mitigation:

- Path-based gates, labels for deep validation, nightly full builds, candidate
  builds on demand.

### APS Metadata Becomes Ceremony

Cause:

- Too many required fields too early.

Mitigation:

- Start with only `id`, `status`, `validation`, `changeType`, and optional
  `releaseNote`.

### Generated Release Notes Are Low Quality

Cause:

- PRs and APS items use implementation language.

Mitigation:

- Require one user-facing sentence only for user-visible changes. Allow manual
  polishing after generation.

### Agents Over-Trust Generated State

Cause:

- APS or release records are stale or malformed.

Mitigation:

- Agents must verify against git SHA, CI run status, tag, and release assets
  before marking shipped.

### Emergency Patch Needs To Bypass Main

Cause:

- `main` contains unreleasable changes despite policy.

Mitigation:

- Permit short-lived `hotfix/*` from the last tag, but require an incident APS
  item and immediate reconciliation into `main`.

## Minimum Viable Operational Model

If radical simplification is desired, use this model:

```text
1. One permanent branch: main.
2. Every task starts from an APS item.
3. Every PR targets main.
4. Every merge must leave main green.
5. Release means tagging a green main SHA.
6. CI builds and publishes from tags.
7. APS shipped state updates only from verified release records.
```

Minimum viable rules:

- Delete the normal use of `dev`.
- Delete normal `release/*` branches.
- Delete back-merge steps.
- Keep `hotfix/*` only for emergencies.
- Keep `./scripts/release.sh`, but do not treat local preflight as authority.
- Add `release-readiness.yml` as the canonical pre-tag gate.
- Generate release notes from merged PRs and APS metadata.
- Treat an unreleasable `main` as an incident, not a normal staging state.

Minimum viable diagram:

```text
APS -> PR -> main -> tag -> release -> verified artefacts -> APS shipped
```

This is the clearest expression of the objective: the fastest possible path from
validated intent to safe production release.

## Concrete Documentation Changes Recommended

Update these files after approval:

- `docs/guides/branching-strategy.md`: replace two-branch model with trunk-first
  model and emergency-only hotfix branches.
- `docs/guides/worktree-policy.md`: reduce permanent worktrees from `main` +
  `dev` to `main` only.
- `docs/guides/release-runbook.md`: remove `dev -> main`, `release/*` as normal
  path, and back-merge sections; add release readiness workflow and
  tag-from-main flow.
- `.claude/skills/release/SKILL.md`: change entry from local preflight
  confirmation to CI release-readiness lookup; remove branch-strategy decision
  as a normal release step.
- `plans/aps-rules.md`: add executable metadata recommendations once the APS
  schema is agreed.
- `RELEASE-PLAN.md`: convert to generated or derived release view, not an
  independent release slate authority.

## Decision Point

The key decision is not "direct promotion or stabilisation branch?". That keeps
the old model.

The real decision is:

```text
Should Anvil continue to pay a permanent branch-complexity tax to maintain dev,
or should it make main continuously releasable and move safety into PR gates,
APS metadata, CI readiness, candidate artefact builds, and fast revert/patch
flows?
```

Recommendation: retire `dev` as a normal workflow branch and make `main` the
single releasable product line.
