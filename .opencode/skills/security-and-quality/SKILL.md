---
name: security-and-quality
description: Full-lifecycle security and quality alert remediation — sweeps Dependabot
  alerts, GitHub Code Quality findings (CodeQL plus Copilot), and/or code
  scanning SAST (CodeQL and other tools), builds a prioritised fix plan with
  research citations, executes fixes with persistent problem-solving, and opens
  draft PRs with detailed reports
---

# Security & Quality Alert Remediation

## Overview

Sweep open GitHub security and quality alerts, research and plan fixes, execute
them with persistent problem-solving, and open draft PRs — one per alert group —
with full reports including research citations.

Three tracks, same five-phase pipeline, track-specific behaviour at each phase:

1. **Dependabot track** — dependency vulnerability alerts (npm, github-actions)
2. **Code quality track** — GitHub Code Quality (CodeQL maintainability/reliability)
   plus Copilot AI quality findings
3. **Code scanning track (SAST)** — CodeQL security alerts and other code-scanning
   tools (Semgrep, third-party SARIF, …)

Quality vs SAST classification, fetch commands, and execution ladders live in
[references/alert-tracks.md](references/alert-tracks.md). Secret scanning is out
of scope.

This is an investigative, problem-solving workflow. Do NOT take shortcuts:

- Try bold upgrades (major version bumps) rather than assuming they will break
- Never treat lock file pins as intentional without evidence
- Research the dependency itself — changelogs, issues, community
- Be willing to replace unmaintained deps with modern alternatives
- Treat alerts as an opportunity to refresh stale corners of the codebase
- For quality and SAST findings, think independently — don't blindly apply
  Copilot diffs, CodeQL help text, or Autofix; write a better fix when they
  are wrong, and never weaken auth or crypto to clear an alert
- Cite every decision with a source link

## When to Use

Four variants; full table, slash-menu aliases, and filters:
[USAGE.md](USAGE.md).

- `/security-and-quality` — dependabot alerts only (default)
- `/security-and-quality quality` — code quality only (alias: `--quality`)
- `/security-and-quality scanning` — SAST only (aliases: `sast`, `--scanning`, `--sast`)
- `/security-and-quality --all` — all three tracks in one sweep
- Slash-menu aliases: `/security-and-quality-quality`,
  `/security-and-quality-scanning`, `/security-and-quality-all`
- Optional filters: severity (`/security-and-quality high`), ecosystem
  (`/security-and-quality npm`), tool (`/security-and-quality tool:CodeQL`),
  specific alert (`/security-and-quality #82`)

## Pipeline

Execute these five phases in order. Do NOT skip phases.

---

### Phase 1 — Discovery

#### 1.1 Determine active tracks

Based on invocation args, activate one or more tracks:

- Default (`/security-and-quality`): dependabot track only
- `quality` / `--quality`: code quality track only
- `scanning` / `sast` / `--scanning` / `--sast`: code scanning track only
- `--all`: all three tracks

#### 1.2 Fetch dependabot alerts (dependabot track)

Fetch all open Dependabot alerts with pagination:

```console
gh api repos/{owner}/{repo}/dependabot/alerts --paginate --jq '.[] | select(.state=="open") | {number, state, dependency: .dependency, severity: .security_advisory.severity, advisory: .security_advisory}'
```

If optional args filter by severity or ecosystem, apply them here.

If zero dependabot alerts are open, report that. If only the dependabot track is
active, exit cleanly.

#### 1.3 Fetch quality and scanning alerts

Follow [references/alert-tracks.md](references/alert-tracks.md). Do not dump
every code-scanning alert into the quality track.

- **Quality track:** fetch GitHub Code Quality findings
  (`/code-quality/findings`) and Copilot AI quality alerts. Classify residual
  CodeQL-in-code-scanning alerts with no security severity/tag as quality.
  If Code Quality is 403/404, report it unavailable and continue with Copilot.
- **Scanning track:** fetch open code-scanning alerts and keep SAST only
  (security severity or `security` tag, plus non-Copilot third-party tools).
  If code scanning is 403, report it disabled and continue with other tracks.

Dedupe overlapping Code Quality and code-scanning copies as specified in the
reference. Apply severity and `tool:` filters after classification. If an
active track has zero findings, report that and continue; exit cleanly if no
tracks still have work. Secret scanning is out of scope.

#### 1.4 Check for prior work

Check for existing open branches or draft PRs from previous runs:

```console
# Dependabot track
git branch -r --list 'origin/dependabot/fix/*'
gh pr list --state open --search "Dependabot Alert Fix" --json number,title,headRefName

# Code quality track
git branch -r --list 'origin/quality/fix/*'
gh pr list --state open --search "Code Quality Fix" --json number,title,headRefName

# Code scanning track
git branch -r --list 'origin/scanning/fix/*'
gh pr list --state open --search "Code Scanning Fix" --json number,title,headRefName
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

Grouping, classification tables, and Copilot vs CodeQL quality handling:
[references/alert-tracks.md](references/alert-tracks.md). Independently assess
each finding; do not blindly apply Copilot diffs or CodeQL help text.
Same-file only — cross-file → Escalate at assessment time. Quality groups never
share a branch with Dependabot or scanning groups.

#### Code scanning track (SAST)

Grouping by `tool.name` + `rule.id`, Autofix review, source→sink reading, and
dismiss reasons: [references/alert-tracks.md](references/alert-tracks.md).
Classify **Apply autofix**, **Fix differently**, **Dismiss**, or **Escalate**.
Auth, crypto, and architecture are Escalate. Do not weaken security controls
to clear an alert.

---

### Phase 3 — Plan Presentation

Present the full sweep plan and **pause for user approval**. Show dependabot,
code quality, and scanning groups separately.

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

**Scanning groups:**

```
Group N: <tool> <rule-id> (<severity>, N alerts)
  Files: <file1>, <file2>
  Alerts:
    - #<n> <file:line> — apply autofix | fix differently: <reason> | dismiss: <reason> | escalate: <reason>
  CWE/tags: <if present>
  Risk: <low / medium / high — dataflow, auth, false-positive chance>
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

```console
git checkout <default-branch>
<test command>  # record output
```

#### 4.2 Execute each approved group

Work through approved groups in the priority order in
[references/alert-tracks.md](references/alert-tracks.md) (scanning critical/high,
then Dependabot critical/high, then remaining scanning and Dependabot, then
quality).

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

The sweep report distinguishes the four terminal states listed in Phase 5:
**fixed**, **escalated**, **dismissed**, **skipped**.

#### 4.3 Dependabot execution

**Branch naming:**

- npm single-alert: `dependabot/fix/<package-name>`
- npm multi-alert: `dependabot/fix/<root-dep>-group`
- actions: `dependabot/fix/actions-<action-name>`

**Fix strategy escalation ladder (attempt cap per group: see Guardrails: scope
limits):**

1. **Direct bump** — Update the version constraint, install, build, test
2. **Lock file override** — Use the package manager's override mechanism to
   force the patched version, verify compatibility
3. **Upstream bump** — Upgrade the direct dependency that pulls in the
   vulnerable transitive dep to a version that resolves it
4. **Major version upgrade** — Bump to the next major, fix breaking API changes
   guided by the migration guide, re-test
5. **Replacement** — Swap for a modern alternative, adapt call sites, re-test.
   If replacement exceeds the scope limits, **escalate** rather than attempt
   (see Guardrails: scope limits)

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

  ```console
  gh api --method PATCH /repos/OWNER/REPO/dependabot/alerts/ALERT_NUMBER -f state='dismissed' -f dismissed_reason='not_used' -f dismissed_comment='Dependency is deprecated and being removed from the project.'
  ```

**Commits:**

- `fix(deps): upgrade <package> to vN.x`
- `fix(deps): replace <old> with <new>`
- Atomic commits — version bump separate from code adaptations

Use the Dependabot draft PR template in
[references/report-templates.md](references/report-templates.md).

#### 4.4 Code quality execution

Branch `quality/fix/<category>` (or `quality/fix/misc`). Same-file only; revert
and escalate if tests fail because behaviour changed. Full steps, Copilot vs
CodeQL quality, and commits:
[references/alert-tracks.md](references/alert-tracks.md).

Use the code quality draft PR template in
[references/report-templates.md](references/report-templates.md). Set **Source**
to Copilot, CodeQL Code Quality, or both.

#### 4.5 Code scanning execution

Branch `scanning/fix/<tool>-<rule-slug>`. Review Autofix; prefer fixing the
source over silencing the sink; never auto-commit Autofix. Same-dataflow
cross-file edits are allowed inside the scanning scope limits. Update tests
that encoded the vulnerability; revert unrelated breakage. Full steps:
[references/alert-tracks.md](references/alert-tracks.md).

Use the code scanning draft PR template in
[references/report-templates.md](references/report-templates.md).

---

### Phase 5 — Sweep Report

Output the final summary using the sweep report template in
[references/report-templates.md](references/report-templates.md).

The sweep report distinguishes four terminal states per track: **fixed** (PR
opened), **escalated** (attempted and abandoned), **dismissed** (dependabot:
deprecated dependency dismissed via the API; scanning: code-scanning dismiss
with a required reason; code quality: finding invalid, documented), **skipped**
(excluded by user).

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

**Scope limits (code scanning track):**

- Same-dataflow edits across files are allowed; auth, crypto, or framework
  rewrites → escalate
- Hard backstop: >10 files, >3 packages, architectural changes
- Do not weaken auth, crypto, or authorisation to clear an alert
- Up to 5 fix attempts per group before escalating
