---
name: council
description:
  Multi-perspective code review — spawns specialist agents in parallel, collects
  structured findings, synthesises a verdict
---

# Council Review

## Target

$ARGUMENTS

## Usage

```
/council <target>
```

Where `<target>` is one of:

- A file path or glob (`src/lib/*.ts`)
- A commit reference (`HEAD`, `abc1234`)
- A branch diff (`main...HEAD`)
- `staged` — review staged changes (`git diff --cached`)
- `recent` — review the last commit (`HEAD~1..HEAD`)
- Empty — defaults to `staged` if there are staged changes, otherwise `recent`

## Council Members

| Agent                  | Lens                                       |
| ---------------------- | ------------------------------------------ |
| `council-reviewer`     | Structured findings (security, correctness) |
| `kernel-maintainer`    | Simplicity, performance, zero bloat        |
| `adversarial-reviewer` | Edge cases, failure modes, abuse vectors   |
| `operations-reviewer`  | Observability, reliability, deploy safety  |
| `pragmatic-lead`       | Velocity, consensus, ship-readiness        |

## Orchestration Protocol

### Step 1: Resolve Target

Determine what to review:

1. If `$ARGUMENTS` is empty:
   - Run `git diff --cached --stat` — if non-empty, target is `staged`
   - Otherwise target is `recent` (last commit)
2. If `$ARGUMENTS` is a file/glob, read those files
3. If `$ARGUMENTS` is a commit ref or range, get the diff

Capture the diff or file contents into a variable for agent prompts.

### Step 2: Spawn Council Members in Parallel

Launch **all five agents concurrently** using the Agent tool. Each agent
receives the same diff/content and returns structured findings.

For each agent, use this prompt template:

```
You are participating in a Council code review.

**Review target:** {target_description}
**Your role:** {agent_role_description}

**Changes to review:**
{diff_or_content}

**Instructions:**
Review the changes through your specialist lens. Produce findings as a JSON
object with this structure:

{
  "agent": "{agent_name}",
  "findings": [
    {
      "severity": "critical|major|minor|nit",
      "category": "security|correctness|edge-case|performance|architecture|style|test-coverage|documentation|observability|reliability",
      "description": "Clear description of the issue",
      "file": "path/to/file",
      "line": 42,
      "suggestion": "Concrete fix or improvement"
    }
  ],
  "verdict": "approve|needs-changes|reject",
  "summary": "One paragraph assessment from your perspective"
}

Only flag real issues. Be specific with file and line references. If no issues
found, return an empty findings array with verdict "approve".
```

Agent-specific focus instructions:

- **council-reviewer**: Full-spectrum review — security, correctness, edge cases,
  architecture, test coverage. You are the primary reviewer.
- **kernel-maintainer**: Focus on unnecessary complexity, bloat, performance
  regressions, and avoidable dependencies. Reject anything that could be simpler.
- **adversarial-reviewer**: Focus on attack vectors, untrusted input, failure
  modes, and "what if" scenarios. Assume the worst.
- **operations-reviewer**: Focus on logging, error messages, failure recovery,
  deploy safety, and production readiness.
- **pragmatic-lead**: Focus on whether this is shippable. Flag only blockers.
  Note if other reviewers are over-engineering their concerns.

### Step 3: Collect and Deduplicate

After all agents return:

1. Parse each agent's JSON response
2. Collect all findings into a unified list
3. Deduplicate — if two agents flag the same file+line with the same category,
   keep the higher-severity one and note which agents agreed
4. Sort by severity: critical > major > minor > nit

### Step 4: Synthesise Verdict

Determine the overall verdict:

| Condition                               | Verdict        |
| --------------------------------------- | -------------- |
| Any agent returns `reject`              | **reject**     |
| Any critical or major findings exist    | **needs-changes** |
| Only minor/nit findings                 | **approve** (with notes) |
| No findings                             | **approve**    |

### Step 5: Report

Output the council report in this format:

```markdown
## Council Review: {target}

**Verdict: {APPROVE | NEEDS CHANGES | REJECT}**
**Reviewed by:** council-reviewer, kernel-maintainer, adversarial-reviewer, operations-reviewer, pragmatic-lead

### Critical ({n})
- [{category}] {description} — `{file}:{line}` (flagged by: {agents})
  **Fix:** {suggestion}

### Major ({n})
- [{category}] {description} — `{file}:{line}` (flagged by: {agents})
  **Fix:** {suggestion}

### Minor ({n})
- [{category}] {description} — `{file}:{line}` (flagged by: {agents})

### Nits ({n})
- [{category}] {description} — `{file}:{line}`

### Agent Summaries

**council-reviewer:** {summary}
**kernel-maintainer:** {summary}
**adversarial-reviewer:** {summary}
**operations-reviewer:** {summary}
**pragmatic-lead:** {summary}
```

### Step 6: Handle Findings

After presenting the report, ask the user how to proceed:

- **fix** — Apply fixes for critical and major findings, then re-run council on
  the changed files only
- **defer** — File remaining findings as GitHub issues or APS work items using
  `forge-defer.sh` if available
- **dismiss** — Acknowledge and move on (only valid if no critical findings)
- **commit** — Proceed to commit (only valid if verdict is approve)

## Configuration

| Variable                   | Default | Description                     |
| -------------------------- | ------- | ------------------------------- |
| `CLAUDE_COUNCIL_MEMBERS`   | all 5   | Comma-separated agent list      |
| `CLAUDE_COUNCIL_AUTO_FIX`  | false   | Auto-fix critical/major         |
| `CLAUDE_COUNCIL_SKIP_NITS` | true    | Suppress nit-level findings     |

## Notes

- Council is heavier than `/review` — use it for significant changes, pre-merge
  reviews, or release prep
- For quick single-perspective reviews, use `/review` instead
- Council findings can feed into the Forge pipeline if deferred
