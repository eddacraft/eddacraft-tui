<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# GitHub Actions YAML Governance Surface (Track 3)

| ID      | Owner | Status |
| ------- | ----- | ------ |
| SURFGHA | —     | Draft  |

**Last reviewed:** 2026-04-26

## Purpose

Bring GitHub Actions workflow YAML to **T2 (Policy)** per
[2026-04-08 Language and Coverage Design](../specs/2026-04-08-language-and-coverage-design.md)
§5.2, §8.3 row 2 — pattern catalogue + suppression + policy hook + drift
baseline. Demand: 2 confirmed (Anvil + User B), assumed universal across
early access. Blast radius: **critical** — supply-chain compromise is the
canonical "one ungoverned file ruins everything" case.

Phase 2 deliverable (spec §9 step 7).

## In Scope

- File detection: `.github/workflows/*.yml`, `*.yaml`.
- Pattern catalogue (per spec §8.3 row 2):
  - `pull_request_target` with write permissions
  - `workflow_run` reaching into forks
  - Unpinned action refs (`@main` / `@master` / `@branch-name`)
  - `secrets:` exposed via `env:` to untrusted code
  - Default `GITHUB_TOKEN` granted write permissions implicitly
  - Self-hosted runners on public repos
- Suppression syntax: `# @anvil-ignore <ID>: <reason>`.
- Policy hook integration.
- Drift baseline default-on for `.github/workflows/*.yml`.
- Acceptance per council §16.5 #9: FP rate < N% on Anvil's repo AND
  ≥ 1 external codebase validation.

## Out of Scope

- Reusable workflow graph analysis (workflow → workflow inclusion graph).
- Action source code analysis (the actions themselves).
- Other CI YAML formats (Buildkite, CircleCI, GitLab) — explicitly cut
  per spec §13.
- Job dependency / matrix analysis.

## Interfaces

**Depends on:**

- Existing OPA pipeline.
- [`operational-supplement`](./operational-supplement.aps.md) — check
  registry, drift schema versioning, per-track feature flag, file-presence
  guard.
- Rust suppression parser per
  [ADR-029](../decisions/029-suppression-parser-authority.md) — `#`
  comment style is already supported.

**Exposes:**

- GH Actions pattern catalogue — reference for the supply-chain
  governance story.

## Prerequisites

- OPSUP slices needed for surfaces landed (see SURFSQL prerequisites —
  same set).
- [ADR-029](../decisions/029-suppression-parser-authority.md) Accepted.

## Ready Checklist

Change status to **Ready** when:

- [ ] OPSUP slices landed.
- [ ] ADR-029 Accepted.
- [ ] Anvil's own workflows baselined.
- [ ] External codebase validation candidate identified.
- [ ] Owner named.

## Work Items

Anticipated:

- SURFGHA-001: Workflow file detection.
- SURFGHA-002: Permission/trigger pattern catalogue
  (`pull_request_target`, `workflow_run`, default token writes).
- SURFGHA-003: Action-pinning rules.
- SURFGHA-004: Secret-handling rules.
- SURFGHA-005: Self-hosted runner rules.
- SURFGHA-006: Suppression + policy hook + drift baseline wiring.
- SURFGHA-007: Anvil + external validation runs.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Action-pinning rule trips on `@v1` major-version pins commonly used | Medium | Allowlist action sources that publish stable major tags (e.g. `actions/*`, `docker/*`) |
| `pull_request_target` legitimate uses (e.g. label automation) flagged | Medium | Require `@anvil-ignore` with explicit justification — that is the policy, not a bug |
| Drift baseline floods on first run for established repos | Medium | Pre-baseline; accept as "drift baseline established" event |

## Open Questions

- [ ] Composite-action and reusable-workflow inclusion — flag pinning at
      the inclusion site only, or recurse?
- [ ] How does this interact with Dependabot's own action-version PRs?
