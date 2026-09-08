# Alert tracks — fetch, classify, execute

Track-specific GitHub alert handling for `security-and-quality`. The parent skill owns the five-phase pipeline, approval checkpoint, and guardrails. This file owns fetch commands, the classifier, grouping, assessment, and execution details for the **code quality** and **code scanning (SAST)** tracks.

Prefer `gh api`. GitHub MCP tools (`list_code_scanning_alerts`, `get_code_scanning_alert`, `get_code_quality_finding`) are optional acceleration, not a substitute for the `gh` contract below.

Resolve `{owner}/{repo}` from `gh repo view --json nameWithOwner`.

---

## Classifier

GitHub splits quality and SAST across two products. Do not dump every code-scanning alert into one track.

**Quality track sources (union, then dedupe):**

1. GitHub Code Quality CodeQL findings — `GET /repos/{owner}/{repo}/code-quality/findings`
2. Copilot AI quality — code-scanning alerts whose `tool.name` is Copilot-like (`GitHub Copilot`, or name contains `Copilot`)
3. Residual CodeQL-in-code-scanning quality — code-scanning alerts from `CodeQL` with **no** `rule.security_severity_level` and **no** `security` tag (legacy `security-and-quality` query suite)

**Scanning (SAST) track sources:**

- Code-scanning alerts with `rule.security_severity_level` set, **or** `rule.tags` containing `security`
- Any other code-scanning tool that is not Copilot-like (Semgrep, Checkmarx, third-party SARIF, …)

**Dedupe:** if the same `rule.id` + path + start line appears in both `/code-quality/findings` and code-scanning, keep the Code Quality finding and drop the code-scanning copy.

**Out of scope:** secret scanning. Do not fetch `/secret-scanning/alerts`.

---

## Fetch

### Code Quality (CodeQL) findings

```console
gh api -H "X-GitHub-Api-Version: 2026-03-10" \
  "repos/{owner}/{repo}/code-quality/findings?state=open" --paginate
```

On **403** or **404**, report "GitHub Code Quality not enabled or not available" for this source and continue with Copilot / residual quality. Do not treat that as a sweep failure.

Per finding, keep: `number`, `rule.id`, `rule.title`, `rule.severity` (`error` / `warning` / `note`), `rule.category` (`maintainability` / `reliability`), `rule.help`, `location.path`, line range, `message.text`.

There is no documented dismiss or autofix REST endpoint for these findings. Close path is a fix that disappears on rescan; invalid findings are documented in the plan/PR, not silently PATCHed.

### Copilot AI quality (code-scanning)

```console
gh api "repos/{owner}/{repo}/code-scanning/alerts?tool_name=GitHub+Copilot&state=open" --paginate
```

If empty, list distinct tool names and pick Copilot-like ones:

```console
gh api "repos/{owner}/{repo}/code-scanning/alerts?state=open" --paginate --jq '[.[].tool.name] | unique'
```

If none, report "No Copilot code quality findings" for this source. Continue with Code Quality findings if any.

Per alert, keep: rule id/description, severity, file path and line range, suggested diff from the most recent instance, surrounding code.

### Code scanning SAST

```console
gh api "repos/{owner}/{repo}/code-scanning/alerts?state=open" --paginate
```

Run the classifier. Scanning-track alerts only. If GHAS / code scanning is **403**, report "Code scanning not enabled" for this track and continue with other active tracks.

Per alert, keep: `number`, `tool.name`, `rule.id`, `rule.tags`, `rule.security_severity_level`, `rule.severity`, `rule.help` / `help_uri`, location, message, CWE tags, `most_recent_instance.classifications` (`generated` / `test` / `library` inform dismiss vs fix).

Optional autofix (review, never auto-commit):

```console
gh api -X POST "repos/{owner}/{repo}/code-scanning/alerts/{n}/autofix"
gh api "repos/{owner}/{repo}/code-scanning/alerts/{n}/autofix"
```

Statuses: `pending` (wait once, re-GET), `success` (treat as a suggestion), `error` / `outdated` (write the fix yourself). Do **not** call `.../autofix/commits`.

---

## Grouping

**Quality:** group by source then category. Copilot: Copilot category (e.g. missing error handling). CodeQL quality: `rule.category` then `rule.id`. Singleton Copilot findings still bundle into `quality/fix/misc`. Do not mix Copilot and CodeQL quality in the same group.

**Scanning:** group by `tool.name` + `rule.id`. A group of one is still a group. Same CWE across different rule ids stays separate unless research shows one root cause in the same files — then merge and record why.

Quality groups, scanning groups, and Dependabot groups never share a branch or PR.

---

## Assessment

### Quality

Read the file and surrounding context. Independently assess; do not blindly apply Copilot diffs or CodeQL help text.

| Classification       | Meaning                                                   |
| -------------------- | --------------------------------------------------------- |
| **Apply suggestion** | Copilot diff or equivalent local fix is correct           |
| **Fix differently**  | Valid finding; write a better same-file fix and note why  |
| **Dismiss**          | Invalid, or the code is intentional                       |
| **Escalate**         | Cross-file, behaviour-changing, or needs domain knowledge |

If any fix would require changes outside the source file, classify **Escalate** at assessment time.

Copilot dismissals are documented in the plan/PR (code-scanning dismiss API only when the user approved the plan and the finding is truly invalid). Code Quality findings: document only.

### Scanning (SAST)

Read the flagged sink **and** the dataflow/source when `rule.help` or tags indicate source→sink. Check classifications (`test` / `generated` / `library`) before treating as production.

| Classification      | Meaning                                                                           |
| ------------------- | --------------------------------------------------------------------------------- |
| **Apply autofix**   | Autofix exists and is a correct, complete remediation                             |
| **Fix differently** | Valid issue; write the real fix (often sanitise/validate at source, not the sink) |
| **Dismiss**         | False positive, used in tests, mitigated, or won't fix — with a documented reason |
| **Escalate**        | Auth, crypto, architecture, or dataflow beyond the scanning scope limits          |

Dismiss via the code-scanning API only after plan approval:

```console
gh api --method PATCH "repos/{owner}/{repo}/code-scanning/alerts/{n}" \
  -f state='dismissed' -f dismissed_reason='false positive' \
  -f dismissed_comment='<reason>'
```

`dismissed_reason` must be one of: `false positive`, `won't fix`, `used in tests`, `mitigated`. Never dismiss a SAST alert without a reason string.

---

## Execution

### Quality — `quality/fix/<category>`

Same-file only. After each file: build + test on affected packages. Tests failing because the fix changed observable behaviour → revert that finding, reclassify Escalate, continue the group.

Commits: `fix(quality): <category> improvements` or `fix(quality): address <finding>`.

PR template: code quality draft PR in `report-templates.md`. Set **Source** to the actual source (Copilot, CodeQL Code Quality, or both listed per finding).

### Scanning — `scanning/fix/<tool>-<rule-slug>`

Example: `scanning/fix/codeql-js-sql-injection`.

1. Read source and sink. Prefer fixing the source (validate/encode/authorise) over silencing the sink.
2. If autofix is classified **Apply autofix**, apply it locally and re-read. If it is incomplete, switch to **Fix differently**.
3. Allow same-dataflow edits across files **inside the scanning scope limits**.
4. After each alert in the group: build + test on affected packages.
5. If tests fail because they encoded the vulnerable behaviour, update those tests as part of the fix. If unrelated behaviour broke, revert that alert, reclassify Escalate, continue.
6. Do not weaken auth, crypto, or authorisation to clear an alert.

Commits: `fix(security): <rule-id> <brief>`.

PR template: code scanning draft PR in `report-templates.md`.

---

## Priority (when multiple tracks run)

1. Scanning critical / high (`security_severity_level`)
2. Dependabot critical / high
3. Scanning medium
4. Remaining Dependabot
5. Quality `error`, then `warning` / `note`
6. Remaining scanning (low / note)
