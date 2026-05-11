# CI/CD And Validation Operating Model Specification

Date: 2026-05-10

Status: Proposed

Related authority:

- `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`
- `plans/modules/operating-model-migration.aps.md`
- `plans/archive/modules/release-orchestration.aps.md`
- `plans/modules/documentation-governance.aps.md`

## Purpose

Define the target CI/CD, validation, and execution-pipeline architecture for
Anvil.

The objective is the fastest and cheapest possible path from validated intent to
safe production execution. CI must become thinner, more targeted, more
authoritative, and less wasteful. Local deterministic validation must become
stronger. Agents provide judgement and orchestration assistance, not validation
truth.

This specification is a specialist implementation layer under the Plan / Build /
Release operating model. It does not redefine APS, branching, release authority,
or shipped-state semantics.

## Executive Summary

The current ecosystem already has useful cost controls: path detection,
docs-only fast paths, affected Nx execution, Rust path filtering, cancellation,
and nightly matrix separation. It is still too CI-heavy for the target operating
model.

The main waste is structural:

- Routine PR and push events trigger multiple independent workflows.
- Each workflow repeats checkout, setup, dependency installation, cache restore,
  and tool installation.
- Coverage, security, release planning, CodeQL, Rust checks, Node checks, and
  release gates duplicate validation responsibility.
- CI is still used as an expensive feedback loop for issues that deterministic
  local scripts and hooks should catch earlier.
- Change detection exists, but each workflow interprets risk independently.

Target architecture:

```text
APS intent
  -> deterministic local validation
  -> targeted agent review
  -> fast PR CI
  -> integration SHA validation
  -> release candidate readiness
  -> tag publish
  -> post-publish verification and release record
```

Non-target architecture:

```text
every PR or push
  -> many broad workflows
  -> repeated setup
  -> coverage and security everywhere
  -> full matrices by habit
  -> release judgement embedded in CI logs
```

## Migration Constraint

The target operating model is `main`-first, but the executable repository remains
`dev`-first until `OPMODEL-012` completes.

Therefore:

- Normal work continues to target `dev` until `OPMODEL-012` changes branch
  authority.
- This CI/CD model may introduce target-state concepts now, but executable
  workflows must preserve the current `dev` integration path.
- Any workflow that accepts `main` as the release-readiness branch before
  cutover must clearly label that path as target-state or release/hotfix-specific.
- Recommendations must not silently convert ordinary feature work to `main`.

## Authority Boundaries

| Concern | Authority | CI/CD Role |
| --- | --- | --- |
| Intent, scope, dependencies, acceptance criteria | APS | Validate metadata and drift, never invent work authority. |
| Code history | Git | Validate exact commit SHAs. |
| Fast feedback | Local deterministic scripts/hooks | Mirror commands and record evidence, not replace local discipline. |
| Validation truth | CI result for a commit SHA | Provide authoritative evidence for merges and releases. |
| Review judgement | Targeted agents/council/humans | Consume review evidence, do not run LLMs as deterministic gates. |
| Release source | Annotated tag | Build and publish only from immutable tag inputs. |
| Distributed artefacts | GitHub Release assets | Verify assets and checksums. |
| Shipped-state reconciliation | Release record | Emit/consume records; do not infer shipped state from logs. |

Rule inherited from the agentic execution architecture:

```text
hooks/scripts/CI provide deterministic enforcement
agents provide judgement and orchestration assistance
```

## Current-State Assessment

Observed workflow surfaces:

- `.github/workflows/ci.yml`
- `.github/workflows/rust.yml`
- `.github/workflows/security.yml`
- `.github/workflows/codeql.yml`
- `.github/workflows/release.yml`
- `.github/workflows/napi.yml`
- `.github/workflows/infra.yml`
- `.github/workflows/bench.yml`
- `.github/workflows/ci-nightly.yml`
- `.github/workflows/bench-nightly.yml`
- `.github/workflows/pr-base-guard.yml`
- `.github/workflows/labeler.yml`
- `.github/actions/setup-workspace/action.yml`
- `.github/actions/detect-changes/action.yml`
- `.husky/pre-commit`
- `scripts/release/*.sh`
- `package.json` scripts
- `nx.json` affected and cache configuration

What works:

- Pull request runs cancel stale runs by concurrency group.
- Docs-only PRs avoid the heavy Node jobs in `ci.yml`.
- Rust workflow uses path filtering and an additional Rust-change detector for
  cross-compile gating.
- Nx affected execution exists for TypeScript and Rust project targets.
- Cross-platform Node tests have largely moved to nightly.
- Release artefact publishing is tag-triggered.
- NAPI cross-platform work is path-scoped.
- Precommit is deterministic and cheap.

Current weaknesses:

- CI, Rust, Security, CodeQL, Release, Label, Copilot review, and dynamic CodeQL
  can all run on a single PR sync.
- Pushes to `dev` rerun broad checks after PR checks, instead of running a
  distinct integration-readiness contract.
- Routine TypeScript unit tests run coverage instrumentation and upload coverage
  artefacts.
- Rust push validation runs strict tests and may run coverage instrumentation as
  a second test pass.
- Rust check, test, clippy, format, hakari, deny, acknowledgements, and smoke
  checks are split across many jobs, each paying setup overhead.
- Security checks duplicate dependency, licence, secret, Semgrep, CodeQL, and
  cargo-deny concerns across workflows.
- The release skill/runbook target architecture now expects deterministic
  `scripts/release/*` commands; RELORCH landed that command surface and removed
  the legacy single-file runner.
- Change classification exists, but there is no single shared risk classifier
  consumed by hooks, agents, scripts, and CI.

## Cost And Waste Analysis

The repository reportedly consumed approximately 50,000 CI build minutes in the
first seven days of the month. Static workflow shape and recent run history both
support the conclusion that the problem is systemic.

Waste categories:

| Category | Current Pattern | Target Pattern |
| --- | --- | --- |
| Trigger duplication | Many workflows per PR and push. | One fast PR validation plus selected specialist workflows. |
| Setup duplication | Repeated checkout, toolchain, pnpm install, Azure login, cache restore. | Consolidated jobs or reusable deterministic commands. |
| Coverage overuse | Coverage on routine validation. | Coverage on schedule, candidate, or explicit readiness only. |
| Security overuse | Broad per-PR security workflows. | Path/risk-triggered PR checks plus scheduled full assurance. |
| Matrix overuse | Cross-platform where release confidence is not needed. | Platform matrix only for platform-sensitive changes, candidate, tag, or nightly. |
| Push/PR overlap | PR and integration branch repeat similar gates. | PR proves change; integration proves merged SHA. |
| Release planning leakage | Release planning appears in routine PR workflows. | Candidate workflow owns release readiness. |
| Late validation | CI catches issues after expensive setup. | Hooks/local scripts catch deterministic failures first. |

## Root-Cause Analysis

1. **Validation ownership is fragmented.** Package scripts, workflow YAML,
   release scripts, security tools, docs rules, APS guidance, and agent prompts
   all describe overlapping gates.
2. **Workflow decomposition is job-centric.** Jobs are split by tool, not by
   validation contract or cost/risk class.
3. **CI is used as feedback and authority.** The target model requires CI to be
   authoritative evidence for SHAs, while local deterministic commands provide
   cheap feedback.
4. **Change detection is too coarse and too local.** Each workflow makes its own
   path/risk decision, creating inconsistent execution semantics.
5. **Observability is mostly logs, not decisions.** It is hard to answer which
   checks ran, why they ran, what they cost, and whether they duplicated another
   check.
6. **Migration branch semantics blur responsibilities.** `dev` is still the
   integration branch, while target-state specs increasingly assume `main`.
   Workflows need explicit compatibility mode until cutover.

## Target Validation Layers

| Layer | Purpose | Blocking? | Owner |
| --- | --- | --- | --- |
| Save-time / editor | Fast Anvil checks and formatting hints. | Deterministic safety only. | Anvil/hooks |
| Precommit | Staged formatting, cheap lint, secret-pattern guard, guidance. | Yes for cheap deterministic checks. | Hooks/scripts |
| Local changed preflight | Affected lint/typecheck/test and relevant policy checks. | Required by agent workflow before PR. | Scripts/Nx/Cargo |
| Pre-PR review | Targeted judgement by risk/path. | Blocks only through review policy. | Agents/council |
| Fast PR CI | Format, lint, typecheck, affected tests, metadata checks. | Required. | CI |
| Full PR CI | Full tests/security/platform checks for selected risk. | Required when selected. | CI |
| Integration SHA validation | Prove the merged integration SHA. | Required for integration branch. | CI |
| Candidate readiness | Prove a selected release SHA and candidate metadata. | Required before tag. | CI/RELORCH |
| Tag publish | Build/publish immutable artefacts. | Required. | Release workflow |
| Post-publish | Verify assets/installers/latest pointers and emit release record. | Required. | RELORCH/CI |

## Minimum Viable CI Operating Model

The radically simplified model contains five workflows.

| Workflow | Trigger During Migration | Trigger After `OPMODEL-012` | Purpose |
| --- | --- | --- | --- |
| `validate-pr` | PR to `dev`; release/hotfix PR to `main` | PR to `main` | Fast PR validation and selected full PR checks. |
| `validate-integration` | Push to `dev` | Push to `main` | Validate the merged integration SHA. |
| `assurance-nightly` | Schedule/manual | Schedule/manual | Full security, CodeQL, coverage, matrix, benchmarks. |
| `release-candidate` | Manual/API for explicit `dev` or release SHA | Manual/API for explicit `main` SHA | Release readiness and candidate artefacts. |
| `publish-release` | Tag | Tag | Immutable release build, publish, verify, release record. |

Existing workflow files may be migrated gradually. The architecture is the
contract; filenames are implementation detail until consolidation is safe.

## Trigger Strategy

Fast PR validation runs on every normal PR. Expensive checks run only when path
or risk selection requires them.

Recommended risk selectors:

| Changed Surface | Fast PR | Full PR / Specialist Checks |
| --- | --- | --- |
| `docs/**`, `plans/**`, `*.md` | markdown format/lint, APS/docs metadata | docs governance drift when DOCGOV validators exist |
| `packages/**`, `apps/**`, `tools/**` | affected Node lint/typecheck/test | full Node only for broad dependency or build-system changes |
| `crates/**`, `Cargo.*`, `rust-toolchain.toml` | affected Rust check/test/fmt/clippy | full Rust for workspace-level or release-sensitive changes |
| `policies/**`, policy package | policy tests and security review route | security/adversarial review, full policy suite |
| `.github/**` | workflow lint/guidance | Operations + Pragmatic review, selected dry-run checks |
| `scripts/release*`, `dist-workspace.toml` | release command tests | release-readiness impact, cargo-dist plan |
| `infra/**`, migrations | static validation and preview | Pulumi preview/apply gates as today |
| NAPI binding paths | targeted NAPI checks | NAPI matrix |
| lockfiles/manifests | package manager and dependency checks | Trivy/cargo-deny/licence/acknowledgements |

Push semantics:

- PR CI proves proposed change shape.
- Integration push CI proves the merged SHA.
- Release candidate CI proves a selected release SHA.
- Tag CI publishes immutable artefacts.

These are separate contracts and should not blindly run the same jobs.

## Incremental Execution Model

Add a shared deterministic classifier before major workflow consolidation.

Inputs:

- event type
- base/head SHA
- changed files
- branch/base branch
- APS work item metadata when available
- labels or manual inputs

Labels and manual inputs may escalate validation, request additional checks, or
document an audited operator override. They must not silently downgrade the
path/SHA-derived baseline classification.

Outputs:

```json
{
  "pathClasses": ["rust", "release", "workflow"],
  "riskClasses": ["full", "release", "operations-review"],
  "requiredChecks": ["fast-pr", "rust-affected", "release-plan"],
  "requiredReviews": ["operations", "pragmatic"],
  "warnings": []
}
```

Consumers:

- hooks print concise guidance
- local scripts choose changed validation
- agents route to playbooks and review tiers
- CI enforces required deterministic checks
- PR summaries explain why expensive jobs ran

## Local-First Enforcement

Recommended command surface:

```text
scripts/validate/local.sh --staged
scripts/validate/local.sh --changed
scripts/validate/local.sh --full
scripts/agent/guidance.sh --staged
scripts/agent/guidance.sh --branch
scripts/agent/guidance.sh --pr
```

Recommended package scripts:

```json
{
  "validate:staged": "scripts/validate/local.sh --staged",
  "validate:changed": "scripts/validate/local.sh --changed",
  "validate:full": "scripts/validate/local.sh --full",
  "validate:release": "scripts/release/preflight.sh"
}
```

Precommit remains cheap and deterministic. It may block format, staged lint,
secret-pattern guard, and cheap metadata checks. It must not run council review,
LLM review, release judgement, coverage, full tests, or long-running matrices.

## Cache And Build Optimisation

Priority optimisations:

1. Remove routine TypeScript coverage from fast PR and integration validation.
2. Move Rust coverage to scheduled assurance or release candidate readiness.
3. Consolidate repeated Node setup where wall-clock and runner-minute trade-offs
   favour fewer jobs.
4. Consolidate or target Rust setup-heavy checks where risk allows.
5. Cache installed tool binaries by version: cargo-dist, cargo-deny,
   cargo-hakari, cargo-about, cargo-nextest, cargo-llvm-cov, Regal.
6. Run dependency and licence checks only when manifests, lockfiles, allowlists,
   or acknowledgement tooling changes, plus scheduled assurance.
7. Reserve macOS/Windows matrices for platform-sensitive changes, nightly,
   candidate, and tag workflows.
8. Avoid upload artefacts on routine runs unless they are required evidence.

## Security And Compliance Strategy

Security checks remain important, but they must be targeted.

| Check | PR Trigger | Scheduled Trigger | Release Trigger |
| --- | --- | --- | --- |
| Secret scan | changed diff or high-risk paths | full repository | candidate/tag if secrets-sensitive paths changed |
| Semgrep | source/security paths | full source | candidate for high-risk releases |
| CodeQL | source/security/workflow risk | full language scan | candidate only when required |
| Trivy dependency audit | manifest/lockfile changes | full repository | candidate if dependency changes included |
| cargo-deny | Cargo manifest/lock/config changes | full Rust audit | candidate if Rust dependency changes included |
| Licence/acknowledgements | dependency/licence surfaces | full freshness | release candidate freshness |

## Release Integration

CI release readiness belongs to RELORCH and OPMODEL integration points.

Target release CI phases:

```text
assess -> preflight -> candidate artefacts -> tag -> publish -> verify -> release record
```

During migration, candidate readiness may accept a `dev` integration SHA or a
release/hotfix `main` SHA. After `OPMODEL-012`, normal candidate readiness uses a
green `main` SHA.

Release workflows must distinguish:

- candidate artefacts: non-publishing evidence before tag
- tag artefacts: immutable publishing from a tag
- post-publish verification: install, assets, latest pointers, checksums
- release record: canonical shipped-state evidence for APS reconciliation

## APS Integration

CI should validate APS consistency mechanically without turning APS into a CI log.

Recommended checks:

- PR has APS work item ID or explicit unplanned-work reason.
- Work item validation commands exist or absence is justified.
- Module progress in `plans/index.aps.md` matches module files.
- Changed files align with declared `Files:` metadata when present.
- User-facing changes include release-note metadata when OPMODEL rules land.
- No item is marked shipped without a release record.
- Candidate release metadata maps merged PRs to APS items.

All APS/repo/release drift checks should start warning-only, then become required
once false positives are understood.

## Agent And Council Integration

Agents should reduce CI load by doing earlier deterministic work and earlier
targeted review.

Rules:

- Hooks print deterministic guidance only.
- Agents run local changed validation before opening PRs.
- Agents use the shared classifier to choose review tier.
- Council is risk-triggered, not universal ceremony.
- Review evidence is judgement evidence, not validation proof.
- CI may require that review evidence exists for selected risk classes, but CI
  must not run LLM review itself.

## Observability And Cost Controls

CI must expose why work ran and what it cost.

Minimum telemetry:

- workflow name, event, branch, PR, SHA
- path classes and risk classes
- required checks selected
- skipped checks and reason
- job duration, setup duration, validation duration
- cache hit/miss
- artefact size and retention
- cancellation count
- runner type and cost multiplier
- first failure class

Add a scheduled CI cost report that summarises:

- total runner minutes by workflow/job/event
- cancelled minutes
- matrix minutes by OS
- coverage minutes
- security scan minutes
- release/candidate minutes
- top repeated failure classes
- top duplicated setup costs

## Failure Modes

| Failure Mode | Risk | Mitigation |
| --- | --- | --- |
| CI cost spikes from frequent PR syncs | High | Thin PR CI, cancellation, local preflight. |
| Docs-only work triggers broad workflows | Medium | Shared classifier and required skip semantics. |
| Local and CI commands diverge | High | CI calls the same deterministic scripts where practical. |
| Coverage blocks useful merges | Medium | Coverage becomes observability except candidate policy. |
| Security scan noise blocks unrelated work | Medium | Path/risk triggers and scheduled assurance. |
| Release commands and skill drift | High | RELORCH command harness and startup probe. |
| `main`-first advice violates migration | High | Keep `dev` compatibility until `OPMODEL-012`. |
| APS shipped state drifts | Medium | Release-record-gated reconciliation. |
| Expensive matrices run accidentally | High | Platform-sensitive classifier plus manual override. |

## Rollout Strategy

1. Document authority and create the APS execution module.
2. Add CI cost observability and run-reason summaries.
3. Add shared path/risk classifier in warning mode.
4. Add local validation and agent guidance scripts.
5. Remove routine coverage from fast PR/integration validation.
6. Target dependency, licence, security, and CodeQL checks by path/risk.
7. Consolidate high-overhead jobs where measured runner minutes justify it.
8. Introduce release-candidate readiness for explicit SHAs.
9. Promote warning-only APS/repo/release drift checks to required gates.
10. After `OPMODEL-012`, retarget normal validation from `dev` to `main`.

## Open Questions

- Whether the final consolidated workflow filenames should be adopted in one cut
  or reached through behaviour-only migration first.
- Whether CodeQL dynamic/default runs are organisation-managed outside this
  repository and need an organisation-level setting change.
- Whether remote Nx cache authentication should move to OIDC or another less
  repetitive setup path.
- Whether candidate readiness must prove full coverage or treat coverage as
  advisory release observability.
- Whether CI cost reports should live as GitHub issue comments, generated
  artefacts, or both.

## Recommendation

Adopt this model as the CI/CD and validation specialist architecture. Execute it
through a new cross-cutting APS module that coordinates with OPMODEL, RELORCH,
DOCGOV, council/review work, and existing CI workflows.

The module should not replace `OPMODEL`. It should be inserted into the APS index
as the implementation owner for CI/CD cost, validation layering, shared
classification, local-first enforcement, and pipeline decomposition. OPMODEL
continues to own lifecycle vocabulary, migration sequencing, and `main`-first
cutover safety.
