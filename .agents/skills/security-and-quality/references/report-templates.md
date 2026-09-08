# Security & quality report templates

Use these templates when executing the reporting stages from the parent skill:
the Dependabot draft PR, the code quality draft PR, the code scanning (SAST)
draft PR, and the combined sweep report covering every active track.

## Dependabot draft PR

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

- source title — URL — what it told us
- source title — URL — what it told us

## What was tested

- Build: pass/fail
- Test suite: pass (N tests) / fail (details)
- Affected packages: <list>
- Pre-existing failures (not caused by this change): <list or "none">

## Escalated items

(none, or: description of what needs further discussion)
```

## Code quality draft PR

```markdown
## Code Quality Fix

**Findings addressed:** N findings across M files
**Category:** <error-handling / unused-code / misc>
**Source:** GitHub Copilot AI Code Quality / GitHub Code Quality (CodeQL) / both

## Findings

### <file-path>:<line>

- **Finding:** <what was flagged>
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

## Code scanning draft PR

```markdown
## Code Scanning Fix

**Alerts addressed:** #N, #M
**Tool:** CodeQL / <other SAST tool>
**Rule:** <rule-id>
**Severity:** <security_severity_level or rule.severity>
**CWE/tags:** <if present>

## What was vulnerable

<sink and source in plain English — why this is (or is not) exploitable>

## What was done

- Applied Autofix / fixed differently / dismissed
- <concrete changes: validation, encoding, authorisation, test updates>

## Research sources

- rule help / CWE — URL — what it told us
- source title — URL — what it told us

## What was tested

- Build: pass/fail
- Test suite: pass (N tests) / fail (details)
- Lint: pass/fail
- Affected packages: <list>
- Pre-existing failures (not caused by this change): <list or "none">

## Dismissed alerts

- #<n> <file:line> — <false positive | used in tests | mitigated | won't fix>: <reason>

## Escalated alerts

- #<n> <file:line> — <reason>
```

## Sweep report

```markdown
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

### Code Scanning — Fixed (N alerts across M PRs)

- PR #X: <tool> <rule-id> — alerts #A, #B

### Code Scanning — Escalated (N alerts)

- #<n> <tool> <rule-id> <file:line> — <reason>
  Tried: <what was attempted>
  Needed: <what decision is required>

### Code Scanning — Dismissed (N alerts)

- #<n> <tool> <rule-id> — <false positive | used in tests | mitigated | won't fix>: <reason>

### Code Scanning — Skipped (N alerts)

- #<n> — skipped by user

### Refresh opportunities

- <package> — last release 18 months ago, consider <alternative>
- <package> — deprecated upstream, successor is <new-package>
```
