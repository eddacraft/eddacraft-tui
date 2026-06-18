<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# GitHub Actions YAML Governance Surface (Track 3)

| ID      | Owner      | Status      |
| ------- | ---------- | ----------- |
| SURFGHA | joshuaboys | In Progress |

**Last reviewed:** 2026-06-18

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

Promoted Draft → In Progress 2026-06-18. Checklist satisfied:

- [x] OPSUP slices landed — same set as SURFSQL (OPSUP-001 Done, OPSUP-003
      Merged #2694, OPSUP-005 Merged #2755; `track.surface` umbrella live).
- [x] ADR-029 Accepted — `#` comment style already in the suppression parser.
- [x] Anvil's own workflows baselined — corpus is `.github/workflows/*.yml`
      (~30 workflows); they set explicit `permissions:` and pin to release
      tags, so the PR1 catalogue is clean on them. FP target **N = 1%** (PYLAN
      precedent, operator-ratifiable).
- [x] External codebase validation candidate identified — a popular OSS repo
      with `.github/workflows/` (final pick recorded in SURFGHA-007).
- [x] Owner named — joshuaboys.

## Work Items

Delivered as slices mirroring SURFSQL: library catalogue first, then gate
registration + flag, then validation.

### SURFGHA-001 — Workflow file detection

- **Status:** Merged 2026-06-18 via PR #2773
- **Intent:** Identify the `.github/workflows/*.yml`/`*.yaml` files SURFGHA
  governs.
- **Expected Outcome:** Files under a `.github/workflows/` directory with a
  `.yml`/`.yaml` extension are detected; other YAML (e.g. action metadata,
  non-workflow YAML) is not.
- **Files:** `crates/anvil-checks/src/surface/github_actions/scanner.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::github_actions::scanner::tests::detects_workflow_files_by_path`
- **Confidence:** high

### SURFGHA-002 — Supply-chain pattern catalogue

- **Status:** Merged 2026-06-18 via PR #2773
- **Intent:** Warn on the highest-blast-radius supply-chain risks in workflow
  YAML.
- **Expected Outcome:** Unpinned **branch** action refs (`uses: …@main`/branch,
  not SHA/version-tag), the `pull_request_target` trigger, and self-hosted
  runners are flagged, with `#`-comment awareness and `# @anvil-ignore`
  suppression. Consolidates the anticipated action-pinning (-003) and
  self-hosted (-005) risk families into one catalogue rule.
- **Files:** `crates/anvil-checks/src/surface/github_actions/{scanner,check}.rs`
- **Validation:** `cargo test -p eddacraft-anvil-checks --lib surface::github_actions`
- **Confidence:** high

### SURFGHA-006 — Gate/catalogue registration + flag gating

- **Status:** Merged 2026-06-18 via PR #2776
- **Intent:** Surface SURFGHA in the gate behind `track.surface.gha`.
- **Expected Outcome:** `ANV-SURF-GHA-001` registered + wired into the gate
  dispatcher (warn-only, file-presence guarded), gated behind a
  `track.surface.gha` leaf flag under the OPSUP-005 `track.surface` umbrella,
  opt-in via `ANVIL_TRACK_SURFACE_GHA=1` — exactly the SURFSQL-005 pattern.
- **Validation:** `cargo test -p eddacraft-anvil commands::check_catalog`
- **Dependencies:** SURFGHA-002, OPSUP-005 (Merged)
- **Confidence:** high

### SURFGHA-007 — Anvil + external validation runs

- **Status:** Ready
- **Intent:** Prove the acceptance bar (FP < 1% on Anvil + ≥1 external repo).
- **Validation:** FP report committed under `plans/reviews/`.
- **Dependencies:** SURFGHA-002, SURFGHA-006
- **Confidence:** medium

### Deferred risk families

`workflow_run` reaching into forks, default `GITHUB_TOKEN` write permissions,
and `secrets:` exposed via `env:` (anticipated -002/-004) need permission- and
data-flow resolution that is FP-prone line-by-line; revisit with the
SURFGHA-007 dogfood signal (mirrors the SURFSQL-003 mixed-transaction
deferral).

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
