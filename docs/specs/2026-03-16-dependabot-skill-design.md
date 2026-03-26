# Security & Quality Alert Remediation Skill — Design Spec

- **Date:** 2026-03-16 (updated 2026-03-17)
- **Status:** Draft
- **Skill name:** `dependabot`

**Invocation:**

- `/dependabot` — dependabot alerts only (default, backwards compatible)
- `/dependabot quality` — code quality findings only (alias: `--quality`)
- `/dependabot --all` — both tracks in one sweep
- Optional filters: severity (`/dependabot high`), ecosystem
  (`/dependabot npm`), specific alert (`/dependabot #82`)

## Overview

Full-lifecycle alert remediation skill with two tracks:

1. **Dependabot track** — dependency vulnerability alerts (npm, github-actions)
2. **Code quality track** — GitHub Copilot AI code quality findings

Both tracks follow the same five-phase pipeline (discover → assess → plan →
execute → report) but with track-specific behaviour at each phase.

The skill embodies an investigative, problem-solving approach:

- Try bold upgrades rather than assuming they will break
- Don't treat lock file pins as intentional without evidence
- Research the dependency itself — changelogs, issues, community
- Be willing to replace unmaintained deps with modern alternatives
- Treat alerts as an opportunity to refresh stale corners of the codebase
- For code quality findings, independently assess whether the suggestion is
  correct — write a better fix when the suggested diff is wrong
- Cite every decision with a source link

## Pipeline Phases

### Phase 1 — Discovery

#### 1.1 Determine active tracks

Based on invocation args, activate one or both tracks:

- Default (`/dependabot`): dependabot track only
- `--quality` / `quality`: code quality track only
- `--all`: both tracks

#### 1.2 Fetch dependabot alerts (dependabot track)

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

If zero dependabot alerts are open, report that. If only the dependabot track is
active, exit cleanly.

#### 1.3 Fetch code quality findings (code quality track)

Fetch open Copilot AI code quality findings using the tool name filter:

```bash
gh api "repos/{owner}/{repo}/code-scanning/alerts?tool_name=GitHub+Copilot&state=open" --paginate
```

If that returns empty, the tool name may have changed. List distinct tool names
to confirm:

```bash
gh api repos/{owner}/{repo}/code-scanning/alerts --paginate --jq '[.[].tool.name]' | jq -s 'add | unique'
```

If no Copilot-like tool is found, report "No code quality findings (Copilot not
enabled)" for this track and continue with other tracks if running `--all`. Do
NOT fall back to processing all code scanning alerts — the skill only handles
Copilot AI quality findings, not arbitrary SAST tools.

For each finding, gather:

- Rule ID and description
- Severity
- File path and line range
- Suggested diff (from the alert instance)
- Surrounding code context

Check for existing open PRs or branches from previous runs (`quality/fix/*`). If
a prior draft PR exists for a quality category that still has open findings,
note it for the user in the plan rather than creating a new branch — same
handling as the dependabot track.

#### 1.4 Detect repo setup

Determine the project's tooling:

| Signal              | How to detect                                                        |
| ------------------- | -------------------------------------------------------------------- |
| Package manager     | Lock file: `pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`        |
| Workspace structure | `pnpm-workspace.yaml`, `workspaces` in root `package.json`           |
| Override mechanism  | pnpm: `pnpm.overrides`, npm: `overrides`, yarn: `resolutions`        |
| Build command       | `scripts` in root `package.json`, Nx/Turbo detection                 |
| Test command        | vitest, jest, etc. from scripts or config files                      |
| Default branch      | `gh repo view --json defaultBranchRef --jq '.defaultBranchRef.name'` |

#### 1.5 Gather dependency details (dependabot track)

For each dependabot alert, determine:

- Is the vulnerable package a direct or transitive dependency?
- If transitive, which direct deps pull it in? (`pnpm why <package>` or
  equivalent)
- Which workspace packages are affected? (monorepo only)

### Phase 2 — Assessment & Grouping

#### Dependabot track

Group alerts by root cause. Multiple alerts caused by the same underlying
dependency (e.g., 3 CVEs in `undici` via different paths) become one group.
Separate npm alerts from github-actions alerts — they follow different fix
paths.

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
package name across affected workspace packages), not full code-path analysis
and may miss dynamic or aliased imports and other indirect usages that do not
mention the package name explicitly. Note the result in the report. Still fix if
the upgrade is easy, but deprioritise if it requires significant effort on an
unreachable path.

Groups classified as **Escalate** skip execution entirely and go straight to the
sweep report with their research findings attached.

**Research tools:** Use `gh` CLI for GitHub data (issues, releases, changelogs
on the dependency repo). Use web search for community experience (migration
guides, blog posts). Acknowledge that community research has a staleness caveat
— note when sources are from training knowledge vs live fetches.

#### Code quality track

**Grouping:** Group findings by category first (e.g., all "missing error
handling" findings, all "unused variable" findings). Categories with only one
finding get bundled into a "misc quality fixes" group. Code quality groups are
always separate from dependabot groups — they get their own branches and PRs.

**Assessment:** For each finding, read the file and surrounding context to
independently assess:

1. **Is the finding valid?** — Does the flagged code actually have the issue
   described? Sometimes Copilot flags something that is intentional (e.g., a
   deliberately broad catch clause)
2. **Is the suggested diff correct?** — Read the suggestion, check if it would
   actually improve things or if it misses context
3. **What is the right fix?** — If the suggestion is wrong or incomplete,
   determine the better fix based on understanding the code

Classify each finding:

| Classification       | Meaning                                                          |
| -------------------- | ---------------------------------------------------------------- |
| **Apply suggestion** | Copilot's diff is correct, apply it                              |
| **Fix differently**  | Finding is valid but write a better fix, note why deviated       |
| **Dismiss**          | Finding is invalid or code is intentionally written that way     |
| **Escalate**         | Fix is cross-file, behaviour-changing, or needs domain knowledge |

The scope check happens at assessment time, not execution time. If reading the
context reveals that any fix (suggested or alternative) would require changes
outside the source file, classify as Escalate immediately. This prevents the
plan from promising a fix that execution will immediately abandon.

### Phase 3 — Plan Presentation

Present the full sweep plan and **pause for user approval**. Show dependabot
groups and code quality groups separately.

**Dependabot groups:**

```
Group N: <package-name> (<severity>)
  Alerts: #X, #Y
  Strategy: <strategy>
  Reachability: <reachable / not directly imported>
  Research: <1-2 sentence summary with key source links>
  Risk: <low / medium / high — what could break>
  Scope: <files/packages affected>
```

**Code quality groups:**

```
Group N: <category> (<N findings>)
  Files: <file1>, <file2>
  Findings:
    - <file:line> — <description> — apply suggestion
    - <file:line> — <description> — fix differently: <brief reason>
    - <file:line> — <description> — dismiss: <brief reason>
    - <file:line> — <description> — escalate: <brief reason>
  Risk: <low (single-file, no behaviour change) / medium (non-trivial refactor)>
```

Dismissed and escalated findings are shown in the plan so the user can challenge
classifications before execution begins.

Ask:

> "Here is the fix plan. You can respond with:
>
> - **approve all** or **go** — execute every group as planned
> - **skip group N** or **skip \<name\>** — exclude specific groups
> - **only groups 1, 3** — execute only named groups
> - Or describe modifications
>
> What would you like to do?"

Wait for the user's response. If the user modifies the plan, acknowledge the
changes and proceed without re-presenting the full plan (unless the change is
ambiguous). This is a single approval checkpoint, not an interactive loop.

### Phase 4 — Execution

**Baseline test run:** Before starting any fixes, run the test suite on the
unmodified default branch. Record any pre-existing failures so they can be
distinguished from regressions.

Work through approved groups in priority order. Dependabot groups (especially
critical/high severity) take priority over code quality groups.

For each group:

1. Check for an existing branch or draft PR covering this group (discovered in
   Phase 1). If one exists and the alert is still open, reuse the branch. If the
   existing PR already contains a viable fix, skip and note in sweep report
2. Create a branch from the default branch, or reuse existing from step 1
3. Attempt the fix (track-specific — see below)
4. Validate: build, test, lint, format check, and typecheck on affected packages
5. If it fails, iterate (track-specific escalation)
6. If resolved, open a draft PR targeting the default branch
7. If escalated, document what was tried and why it failed
8. **Clean up on failure** — delete branch if nothing was committed; leave if
   partial commits exist
9. **Continue regardless** — a failed group does not block subsequent groups

Return to the default branch between groups.

The sweep report distinguishes three states: **fixed** (PR opened),
**escalated** (attempted and abandoned), **skipped** (excluded by user).

#### Dependabot execution details

**Fix strategy escalation ladder (up to 5 attempts per group):**

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

**Escalation triggers:**

- Fix requires changing build tooling (e.g., esbuild to vite)
- Fix requires framework migration (e.g., React major version)
- Fix touches more than the dependency's API surface
- No viable upgrade path AND no alternative exists
- Package is being deprecated/removed anyway (flag for dismissal via
  `gh api --method PATCH repos/{owner}/{repo}/dependabot/alerts/{alert_number} -f state=dismissed -f dismissed_reason="not_used" -f dismissed_comment="Package is being deprecated/removed; alert closed per remediation guidelines."`
  with a documented reason and payload that sets the alert state to dismissed)

#### Code quality execution details

For each finding in an approved group:

1. Read the file and understand the surrounding context
2. If classified as **apply suggestion** — apply the suggested diff
3. If classified as **fix differently** — write the better fix, note why the
   suggestion was insufficient
4. If classified as **dismiss** — skip, document the reason
5. Run build + test on affected packages after each file
6. **If tests break** — the fix changed observable behaviour. Revert that
   specific finding's fix, reclassify as escalated, continue with remaining
   findings in the group

**Code quality escalation triggers (stricter than dependabot):**

- Fix requires changes outside the file where the finding lives
- Fix changes observable behaviour (tests fail after the change)
- Hard backstop: >10 files, >3 packages, architectural changes

### Phase 5 — Sweep Report

Final output to terminal summarising the full sweep:

```
## Sweep Complete

### Dependabot — Fixed (N alerts across M PRs)
- PR #X: <package> — <strategy> — alerts #A, #B
- PR #Y: <package> — <strategy> — alert #C

### Dependabot — Escalated (N alerts)
- <package> — <reason> — alerts #D, #E
  Tried: <what was attempted>
  Needed: <what architectural decision is required>

### Dependabot — Dismissed (N alerts)
- <package> — <reason for dismissal>

### Dependabot — Skipped (N alerts)
- <package> — skipped by user

### Code Quality — Fixed (N findings across M PRs)
- PR #X: error-handling — 4 findings fixed, 1 dismissed
- PR #Y: misc — 3 findings fixed differently

### Code Quality — Escalated (N findings)
- <file:line> — <reason>

### Code Quality — Dismissed (N findings)
- <file:line> — <reason>

### Refresh opportunities
- <package> — last release 18 months ago, consider <alternative>
- <package> — deprecated upstream, successor is <new-package>
```

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

### Dependabot PRs

**Branch naming:**

Every alert belongs to exactly one group (even if that group has one alert).
Every group that completes a fix attempt gets exactly one branch and one draft
PR. Groups that were escalated or skipped do not get branches or PRs — they
appear only in the sweep report.

- `dependabot/fix/<package-name>` for single-alert groups
- `dependabot/fix/<root-dep>-group` for multi-alert groups
- `dependabot/fix/actions-<action-name>` for github-actions groups

**Commit conventions:**

- `fix(deps): upgrade \<package\> to vN.x`
- `fix(deps): replace <old> with <new>`
- Atomic commits — version bump separate from code adaptations

**Draft PR description format:**

```markdown
## Dependabot Alert Fix

**Alerts addressed:** #N, #M **Severity:** high **Strategy:** upstream bump

## What was vulnerable

\<package\> via \<dependency chain\> — \<CVE summary in plain English\>

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

### Code Quality PRs

**Branch naming:**

- `quality/fix/<category>` — e.g., `quality/fix/error-handling`,
  `quality/fix/unused-code`
- `quality/fix/misc` — for bundled single findings from different categories

**Commit conventions:**

- `fix(quality): <category> improvements` for grouped fixes
- `fix(quality): address <specific finding>` for singles in misc

**Draft PR description format:**

```markdown
## Code Quality Fix

**Findings addressed:** N findings across M files **Category:** <error-handling
/ unused-code / misc> **Source:** GitHub Copilot AI Code Quality

## Findings

### <file-path>:<line>

- **Finding:** <what Copilot flagged>
- **Action:** applied suggestion / fixed differently / dismissed
- **Rationale:** <why this fix is correct, or why it differs from the
  suggestion>
- **Diff context:** <brief description of the change>

### <file-path>:<line>

...

## What was tested

- Build: pass/fail
- Test suite: pass (N tests)
- Lint: pass/fail
- Affected packages: <list>

## Dismissed findings

- <file:line> — <reason for dismissal>

## Escalated findings

- <file:line> — <reason for escalation>
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
- Test: `pnpm nx run-many --target test --exclude=@eddacraft/anvil-e2e`
  (excludes Playwright e2e targets)
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

**Scope limits (dependabot track):**

- Replacement that touches >10 files or >3 workspace packages → escalate
- Architectural changes (build tools, frameworks) → escalate
- Up to 5 fix attempts per group before escalating

**Scope limits (code quality track — stricter):**

- Fix requires changes outside the source file → escalate
- Fix changes observable behaviour (tests fail) → revert and escalate
- Hard backstop: >10 files, >3 packages, architectural changes

## Key Principles

1. Try the bold path first — don't assume major bumps will break
2. Lock file pins are not intent — investigate before accepting
3. Research the dependency — changelogs, issues, community
4. Replace, don't just bump — unmaintained deps get modern alternatives
5. Alerts are a refresh opportunity — surface stale deps worth modernising
6. Cite your sources — every decision backed by a link
7. Escalate on architecture, persist on everything else
8. For code quality: think independently — don't blindly apply suggestions,
   write a better fix when the suggestion is wrong
9. Code quality escalation is stricter — single-file, no behaviour changes
