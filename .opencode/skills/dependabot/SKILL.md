---
name: dependabot
description: Full-lifecycle security and quality alert remediation — sweeps Dependabot
  alerts and/or Copilot AI code quality findings, builds a prioritised fix plan
  with research citations, executes fixes with persistent problem-solving, and
  opens draft PRs with detailed reports
---

# Security & Quality Alert Remediation

## Overview

Sweep open GitHub security and quality alerts, research and plan fixes, execute
them with persistent problem-solving, and open draft PRs — one per alert group —
with full reports including research citations.

Two tracks:

1. **Dependabot track** — dependency vulnerability alerts (npm, github-actions)
2. **Code quality track** — GitHub Copilot AI code quality findings

Both tracks follow the same five-phase pipeline but with track-specific
behaviour at each phase.

This is an investigative, problem-solving workflow. Do NOT take shortcuts:

- Try bold upgrades (major version bumps) rather than assuming they will break
- Never treat lock file pins as intentional without evidence
- Research the dependency itself — changelogs, issues, community
- Be willing to replace unmaintained deps with modern alternatives
- Treat alerts as an opportunity to refresh stale corners of the codebase
- For code quality findings, think independently — don't blindly apply
  suggestions, write a better fix when the suggestion is wrong
- Cite every decision with a source link

## When to Use

- `/dependabot` — dependabot alerts only (default, backwards compatible)
- `/dependabot quality` — code quality findings only (alias: `--quality`)
- `/dependabot --all` — both tracks in one sweep
- Optional filters: severity (`/dependabot high`), ecosystem
  (`/dependabot npm`), specific alert (`/dependabot #82`)

## Pipeline

Execute these five phases in order. Do NOT skip phases.

$ARGUMENTS

---

### Phase 1 — Discovery

#### 1.1 Determine active tracks

Based on invocation args, activate one or both tracks:

- Default (`/dependabot`): dependabot track only
- `quality` / `--quality`: code quality track only
- `--all`: both tracks

#### 1.2 Fetch dependabot alerts (dependabot track)

Fetch all open Dependabot alerts with pagination:

```bash
gh api repos/{owner}/{repo}/dependabot/alerts --paginate \
  --jq '.[] | select(.state=="open") | {number, state, dependency: .dependency, severity: .security_advisory.severity, advisory: .security_advisory}'
```

If optional args filter by severity or ecosystem, apply them here.

If zero dependabot alerts are open, report that. If only the dependabot track is
active, exit cleanly.

#### 1.3 Fetch code quality findings (code quality track)

Fetch open Copilot AI code quality findings using the exact tool name filter:

```bash
gh api "repos/{owner}/{repo}/code-scanning/alerts?tool_name=GitHub+Copilot&state=open" --paginate
```

If that returns empty, the tool name may have changed. List distinct tool names
to confirm:

```bash
gh api "repos/{owner}/{repo}/code-scanning/alerts?state=open" --paginate \
  --jq '[.[].tool.name] | unique'
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

#### 1.4 Check for prior work

Check for existing open branches or draft PRs from previous runs:

```bash
# Dependabot track
git branch -r --list 'origin/dependabot/fix/*'
gh pr list --state open --search "Dependabot Alert Fix" --json number,title,headRefName

# Code quality track
git branch -r --list 'origin/quality/fix/*'
gh pr list --state open --search "Code Quality Fix" --json number,title,headRefName
```

If a prior draft PR exists for an alert or finding category that is still open,
note it for the user in the plan rather than creating a new branch.

#### 1.5 Detect repo setup

Determine the project's tooling:

| Signal              | How to detect                                                        |
| ------------------- | -------------------------------------------------------------------- |
| Package manager     | Lock file: `pnpm-lock.yaml`, `yarn.lock`, `package-lock.json`        |
| Workspace structure | `pnpm-workspace.yaml`, `workspaces` in root `package.json`           |
| Override mechanism  | pnpm: `pnpm.overrides`, npm: `overrides`, yarn: `resolutions`        |
| Build command       | `scripts` in root `package.json`, Nx/Turbo detection                 |
| Test command        | vitest, jest, etc. from scripts or config files                      |
| Default branch      | `gh repo view --json defaultBranchRef --jq '.defaultBranchRef.name'` |

#### 1.6 Gather dependency details (dependabot track)

For each dependabot alert, determine:

- Is the vulnerable package a direct or transitive dependency?
- If transitive, which direct deps pull it in? (`pnpm why <package>` or
  equivalent)
- Which workspace packages are affected? (monorepo only)

---

### Phase 2 — Assessment & Grouping

#### Dependabot track

**Grouping:** Multiple alerts caused by the same underlying dependency become
one group. Each alert belongs to exactly one group. A group with one alert is
still a group. Separate npm alerts from github-actions alerts — they follow
different fix paths.

**Research each group:**

1. **Vulnerability advisory** — read the CVE details, affected versions, fixed
   versions from the alert data
2. **Dependency chain** — trace to direct deps (already gathered in 1.6)
3. **Dependency repo** — use `gh` to check releases, changelog, migration guide
   on the upstream repo
4. **Community experience** — use web search for migration guides, blog posts,
   known issues with the upgrade path
5. **Maintenance status** — if no releases in 12+ months with open security
   issues unaddressed, search for modern alternatives

All key sources must be collected with URLs for citation in the plan and PR
descriptions. Note when sources come from training knowledge vs live fetches —
training knowledge has a staleness caveat.

**Reachability check:** Grep-based approximation: search for imports of the
vulnerable package across affected workspace packages. This is not full
code-path analysis. Note the result ("reachable" = any import found, "not
directly imported" = no imports of the vulnerable package, may still be used
transitively). Still fix if the upgrade is easy. Deprioritise if it requires
significant effort on an unreachable path.

**Classify strategy:**

| Strategy          | When                                                         |
| ----------------- | ------------------------------------------------------------ |
| **Quick bump**    | Patch/minor update available, low risk                       |
| **Major upgrade** | Breaking changes, needs testing and possibly code changes    |
| **Replace**       | Dependency is unmaintained/legacy, modern alternative exists |
| **Override**      | Transitive dep can be forced via lock file overrides         |
| **Escalate**      | Fix requires architectural changes beyond dependency surface |

Groups classified as **Escalate** skip execution entirely and go straight to the
sweep report with their research findings attached.

**Research tools:** Use `gh` CLI for GitHub data (issues, releases, changelogs
on the dependency repo). Use web search for community experience (migration
guides, blog posts).

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

| Classification       | Meaning                                                                                                                                                                     |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Apply suggestion** | Copilot's diff is correct, apply it                                                                                                                                         |
| **Fix differently**  | Finding is valid but write a better fix, note why deviated. Must be confined to the same file — if the better fix requires cross-file changes, classify as Escalate instead |
| **Dismiss**          | Finding is invalid or code is intentionally written that way                                                                                                                |
| **Escalate**         | Fix is cross-file, behaviour-changing, or needs domain knowledge                                                                                                            |

The scope check happens at assessment time, not execution time. If reading the
context reveals that any fix (suggested or alternative) would require changes
outside the source file, classify as Escalate immediately. This prevents the
plan from promising a fix that execution will immediately abandon.

---

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

---

### Phase 4 — Execution

#### 4.1 Baseline test run

Before starting any fixes, run the test suite on the unmodified default branch.
Record any pre-existing failures so they can be distinguished from regressions.

```bash
git checkout <default-branch>
<test command>  # record output
```

#### 4.2 Execute each approved group

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
9. **Continue regardless** — a failed group does NOT block subsequent groups

Return to the default branch between groups.

The sweep report distinguishes three states: **fixed** (PR opened),
**escalated** (attempted and abandoned), **skipped** (excluded by user).

#### 4.3 Dependabot execution

**Branch naming:**

- npm single-alert: `dependabot/fix/<package-name>`
- npm multi-alert: `dependabot/fix/<root-dep>-group`
- actions: `dependabot/fix/actions-<action-name>`

**Fix strategy escalation ladder (up to 5 attempts per group):**

1. **Direct bump** — Update the version constraint, install, build, test
2. **Lock file override** — Use the package manager's override mechanism to
   force the patched version, verify compatibility
3. **Upstream bump** — Upgrade the direct dependency that pulls in the
   vulnerable transitive dep to a version that resolves it
4. **Major version upgrade** — Bump to the next major, fix breaking API changes
   guided by the migration guide, re-test
5. **Replacement** — Swap for a modern alternative, adapt call sites, re-test.
   If replacement would touch more than 10 files or span more than 3 workspace
   packages, **escalate** rather than attempt

**Lock file pin handling:**

Never assume a pinned version is intentional. Check:

- Is there an explicit override/resolution in `package.json`? → intentional,
  investigate why (git blame, commit messages) before proceeding
- Is it just a lock file resolution artefact? → incidental, proceed with the
  upgrade

**Escalation triggers:**

- Fix requires changing build tooling (e.g., esbuild to vite)
- Fix requires framework migration (e.g., React major version)
- Fix touches more than the dependency's API surface — architectural changes
- No viable upgrade path AND no alternative exists
- Package is being deprecated/removed anyway — flag for dismissal via:

  ```bash
  gh api \
    --method PATCH \
    "/repos/OWNER/REPO/dependabot/alerts/ALERT_NUMBER" \
    -f state="dismissed" \
    -f dismissed_reason="not_used" \
    -f dismissed_comment="Dependency is deprecated and being removed from the project."
  ```

**Commits:**

- `fix(deps): upgrade <package> to vN.x`
- `fix(deps): replace <old> with <new>`
- Atomic commits — version bump separate from code adaptations

**Draft PR description:**

```markdown
## Dependabot Alert Fix

**Alerts addressed:** #N, #M
**Severity:** <severity>
**Strategy:** <strategy used>

## What was vulnerable

<package> via <dependency chain> — <CVE summary in plain English>

## What was done

- <concrete changes made>

## Research sources

- [source title](url) — what it told us
- [source title](url) — what it told us

## What was tested

- Build: pass/fail
- Test suite: pass (N tests) / fail (details)
- Affected packages: <list>
- Pre-existing failures (not caused by this change): <list or "none">

## Escalated items

(none, or: description of what needs further discussion)
```

#### 4.4 Code quality execution

**Branch naming:**

- `quality/fix/<category>` — e.g., `quality/fix/error-handling`,
  `quality/fix/unused-code`
- `quality/fix/misc` — for bundled single findings from different categories

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

**Commits:**

- `fix(quality): <category> improvements` for grouped fixes
- `fix(quality): address <specific finding>` for singles in misc

**Draft PR description:**

```markdown
## Code Quality Fix

**Findings addressed:** N findings across M files
**Category:** <error-handling / unused-code / misc>
**Source:** GitHub Copilot AI Code Quality

## Findings

### <file-path>:<line>

- **Finding:** <what Copilot flagged>
- **Action:** applied suggestion / fixed differently / dismissed
- **Rationale:** <why this fix is correct, or why it differs from the suggestion>
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

---

### Phase 5 — Sweep Report

Output a final summary to the terminal:

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

The sweep report distinguishes three terminal states per track: **fixed** (PR
opened), **escalated** (attempted and abandoned), **skipped** (excluded by
user). Code quality also has **dismissed** (finding invalid, documented).

---

## GitHub Actions Alerts

Actions alerts follow a simpler pipeline than npm alerts. The npm escalation
ladder (lock file overrides, upstream bumps) does not apply.

**Fix path:**

1. Identify the action repo and the vulnerable version
2. Check the action repo for the latest release/tag that fixes the CVE
3. Update the `uses:` version pin in the workflow YAML file(s)
   (e.g., `actions/checkout@v3` → `actions/checkout@v4`)
4. Validate the workflow YAML parses correctly
5. If the action has a major version bump, check the action's changelog for
   breaking changes (new required inputs, removed features, runner version
   requirements)
6. Open a draft PR — adapted format (no build/test results, instead note which
   workflows were updated)

**Escalation:** If the action has no patched release, or the major bump requires
workflow restructuring, escalate.

**Grouping:** Multiple workflow files using the same vulnerable action become one
group.

---

## Guardrails

**Safety rules:**

- Never force-push or modify existing commits
- Never dismiss an alert without documenting the reason
- Never auto-merge — all PRs are draft
- Run the full test suite for affected packages before considering a fix
  successful
- Pre-existing test failures are noted but not counted as regressions from the
  fix

**Git hygiene:**

- One branch per PR, branched from the detected default branch
- Conventional commits
- Atomic commits (version bump separate from code adaptations)
- Return to the default branch between groups

**Scope limits (dependabot track):**

- Replacement that touches >10 files or >3 workspace packages → escalate
- Architectural changes (build tools, frameworks) → escalate
- Up to 5 fix attempts per group before escalating

**Scope limits (code quality track — stricter):**

- Fix requires changes outside the source file → escalate
- Fix changes observable behaviour (tests fail) → revert and escalate
- Hard backstop: >10 files, >3 packages, architectural changes
