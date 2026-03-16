# Dependabot Alert Remediation Skill — Design Spec

**Date:** 2026-03-16 **Status:** Draft **Skill name:** `dependabot`
**Invocation:** `/dependabot` (optional args: severity filter, ecosystem filter)

## Overview

Full-lifecycle Dependabot alert remediation skill. Sweeps all open alerts,
builds a prioritised fix plan with research citations, executes fixes with
persistent problem-solving, and opens draft PRs (one per alert or alert group)
with detailed reports.

The skill embodies an investigative, problem-solving approach rather than
surface-level bumps:

- Try bold upgrades (major version bumps) rather than assuming they'll break
- Don't treat lock file pins as intentional — investigate why before giving up
- Research the dependency itself (changelogs, issues, community)
- Be willing to replace unmaintained deps with modern alternatives
- Treat alerts as an opportunity to refresh stale corners of the codebase

## Pipeline Phases

### Phase 1 — Discovery

Fetch all open Dependabot alerts via `gh api` (handle pagination for repos with
many alerts). For each alert, gather:

- Severity (critical, high, medium, low)
- Package name and ecosystem (npm, github-actions, etc.)
- Manifest path
- Whether it's a direct or transitive dependency
- What direct deps pull it in (dependency chain)

Check for existing open PRs or branches from previous runs (`dependabot/fix/*`)
to avoid duplicate work. If a prior draft PR exists for an alert that is still
open, note it for the user in the plan rather than creating a new branch.

If zero alerts are open, report that and exit cleanly.

Detect the repo setup:

- Package manager (pnpm, npm, yarn)
- Workspace structure (monorepo or single-package)
- Override mechanism (pnpm overrides, npm overrides, yarn resolutions)
- Build and test commands
- Default branch via `gh repo view --json defaultBranchRef`

### Phase 2 — Assessment & Grouping

Group alerts by root cause. Multiple alerts caused by the same underlying
dependency (e.g., 3 CVEs in `undici` via different paths) become one group.

For each group, research:

1. Vulnerability advisory — CVE details, affected versions, fixed versions
2. Dependency chain — trace to direct deps
3. Dependency repo — changelog, release notes, migration guide
4. Community experience — GitHub issues, discussions, blog posts about the
   upgrade path
5. Maintenance status — if no releases in 12+ months with open security issues,
   search for modern alternatives

Classify each group into a strategy:

| Strategy          | When                                                         |
| ----------------- | ------------------------------------------------------------ |
| **Quick bump**    | Patch/minor update available, low risk                       |
| **Major upgrade** | Breaking changes, needs testing and possibly code changes    |
| **Replace**       | Dependency is unmaintained/legacy, modern alternative exists |
| **Override**      | Transitive dep can be forced via lock file overrides         |
| **Escalate**      | Fix requires architectural changes beyond dependency surface |

Perform a reachability check: does the project import or use the vulnerable
package directly? This is a grep-based approximation (search for imports of the
package name across affected workspace packages), not full code-path analysis.
Note the result in the report. Still fix if the upgrade is easy, but
deprioritise if it requires significant effort on an unreachable path.

Groups classified as **Escalate** in this phase skip execution entirely and go
straight to the sweep report with their research findings attached.

**Research tools:** Use `gh` CLI for GitHub data (issues, releases, changelogs
on the dependency repo). Use web search for community experience (migration
guides, blog posts). Acknowledge that community research has a staleness caveat
— note when sources are from training knowledge vs live fetches.

### Phase 3 — Plan Presentation

Present the full sweep plan for user approval before any changes. Each group
shows:

- Alerts covered (by number)
- Proposed strategy
- Research findings with source links (URLs to changelogs, issues, guides)
- Risk assessment
- Expected scope of changes

The skill pauses and asks the user for approval. The user can respond with:

- **"approve all"** or **"go"** — execute every group as planned
- **"skip group N"** or **"skip \<package\>"** — exclude specific groups
- **"only groups 1, 3"** — execute only named groups
- **Modification requests** — e.g., "try replacement instead of bump for group
  2"

If the user modifies the plan, the skill acknowledges the changes and proceeds
without re-presenting the full plan (unless the change is ambiguous). This is a
single approval checkpoint, not an interactive loop.

### Phase 4 — Execution

**Baseline test run:** Before starting any fixes, run the test suite on the
unmodified default branch. Record any pre-existing failures so they can be
distinguished from regressions caused by dependency changes.

Work through approved groups in priority order (critical/high first).

For each group:

1. Create a branch from the default branch (detected in Phase 1)
2. Attempt the fix
3. Build and run tests on affected packages
4. If it fails, iterate using the fix strategy escalation ladder
5. If resolved, open a draft PR targeting the default branch
6. If escalated, document what was tried and why it failed
7. **Clean up on failure** — if the group is abandoned, delete the branch if
   nothing was committed; leave it if partial commits exist (for reference)
8. **Continue regardless** — a failed group does not block subsequent groups

The sweep report distinguishes three states: **fixed** (PR opened),
**escalated** (attempted and abandoned), **skipped** (excluded by user).

**Fix strategy escalation ladder (up to 3-4 attempts per group):**

1. **Direct bump** — Update version constraint, install, build, test
2. **Lock file override** — Use pnpm overrides (or npm/yarn equivalent) to force
   the patched version, verify compatibility
3. **Upstream bump** — Upgrade the direct dependency that pulls in the
   vulnerable transitive dep
4. **Major version upgrade** — Bump to next major, fix breaking API changes
   guided by migration guide, re-test
5. **Replacement** — Swap for a modern alternative, adapt call sites, re-test.
   If replacement would touch more than 10 files or span more than 3 workspace
   packages, escalate rather than attempt — the scope is architectural

**Lock file pin handling:**

Never assume a pinned version is intentional. Check whether the pin comes from a
resolution override in `package.json` (intentional) or is just a lock file
artifact (incidental). If incidental, proceed with the upgrade. If intentional,
investigate why (git blame, commit messages) before deciding.

**Escalation triggers (stop and report, don't attempt):**

- Fix requires changing build tooling (e.g., esbuild to vite)
- Fix requires framework migration (e.g., React major version)
- Fix touches more than the dependency's API surface
- No viable upgrade path AND no alternative exists
- Package is being deprecated/removed anyway (flag for dismissal via
  `gh api --method PATCH` with a documented reason)

### Phase 5 — Sweep Report

Final output to terminal summarising the full sweep:

- Draft PRs created (with links)
- Alerts resolved per PR
- Alerts escalated with reasons and what was attempted
- Refresh opportunities spotted (legacy deps worth replacing)

## GitHub Actions Ecosystem

Actions alerts follow a simpler pipeline than npm alerts. The npm escalation
ladder (lock file overrides, upstream bumps) does not apply.

**Actions fix path:**

1. Identify the action repo and the vulnerable version
2. Check the action repo for the latest release/tag that fixes the CVE
3. Update the `uses:` version pin in the workflow YAML file(s) (e.g.,
   `actions/checkout@v3` → `actions/checkout@v4`)
4. Validate the workflow YAML parses correctly (`yq` or syntax check)
5. If the action has a major version bump, check the action's changelog for
   breaking changes (new required inputs, removed features, runner version
   requirements)
6. Open a draft PR with the same reporting format (adapted: no build/test
   results, instead note which workflows were updated)

**Actions escalation:** If the action has no patched release, or the major bump
requires workflow restructuring (e.g., a composite action replaced by a
different approach), escalate.

**Grouping:** Actions alerts are grouped separately from npm alerts. Multiple
workflow files using the same vulnerable action become one group.

## PR Structure

**Branch naming:**

Every alert belongs to exactly one group (even if that group has one alert).
Every group gets exactly one branch and one draft PR.

- `dependabot/fix/<package-name>` for single-alert groups
- `dependabot/fix/<root-dep>-group` for multi-alert groups
- `dependabot/fix/actions-<action-name>` for github-actions groups

**Commit conventions:**

- `fix(deps): upgrade <package> to vN.x`
- `fix(deps): replace <old> with <new>`
- Atomic commits — version bump separate from code adaptations

**Draft PR description format:**

```markdown
## Dependabot Alert Fix

**Alerts addressed:** #N, #M **Severity:** high **Strategy:** upstream bump

## What was vulnerable

<package> via <dependency chain> — <CVE summary in plain English>

## What was done

- Upgraded <direct-dep> from vX to vY (pulls in patched <transitive-dep>)
- Updated import path in <file> due to breaking API change
- <any other changes>

## Research sources

- [changelog entry](url) — breaking changes in vY
- [GitHub issue](url) — confirms compatibility
- [migration guide](url) — API surface changes

## What was tested

- Full build: pass/fail
- Test suite: pass (N tests)
- Affected packages: <list of workspace packages>

## Escalated items

(none, or description of what needs architectural discussion)
```

## Repo Detection

**Generic detection (runs on startup):**

| Signal              | Detection                                              |
| ------------------- | ------------------------------------------------------ |
| Package manager     | Lock file presence: pnpm-lock.yaml, yarn.lock, etc.    |
| Workspace structure | pnpm-workspace.yaml, workspaces in package.json, lerna |
| Override mechanism  | pnpm overrides, npm overrides, yarn resolutions        |
| Build command       | scripts in package.json, Nx/Turbo detection            |
| Test command        | vitest, jest, etc.                                     |

**Monorepo behaviour:**

- Trace which workspace packages are affected by a vulnerable dep
- Run targeted builds/tests on affected packages, not full rebuild
- Check whether deps are declared in root, workspace packages, or both
- Understand lock file pins in monorepos are almost always artifacts

**EddaCraft/anvil-001 specialisation:**

- pnpm workspaces with Nx
- Build: `pnpm nx run-many --target=build` or `pnpm nx run <project>:build`
- Test: `pnpm vitest run` (direct vitest, not via Nx)
- Overrides: root `package.json` under `pnpm.overrides`
- Two ecosystems: npm + github-actions (actions alerts update workflow YAML
  version pins)

## Guardrails

**Safety rules:**

- Never force-push or modify existing commits
- Never dismiss an alert without documenting the reason
- Never auto-merge — all PRs are draft
- Run full test suite for affected packages before considering a fix successful
- Pre-existing test failures are noted but not counted as regressions

**Git hygiene:**

- One branch per PR, branched from the detected default branch
- Conventional commits
- Atomic commits (version bump separate from code changes)

## Key Principles

1. Try the bold path first — don't assume major bumps will break
2. Lock file pins are not intent — investigate before accepting
3. Research the dependency — changelogs, issues, community
4. Replace, don't just bump — unmaintained deps get modern alternatives
5. Alerts are a refresh opportunity — surface stale deps worth modernising
6. Cite your sources — every decision backed by a link
7. Escalate on architecture, persist on everything else
