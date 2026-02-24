---
name: forge
description:
  Orchestrate a Forge pre-commit review — spawns forge-reviewer, runs
  negotiation rounds, applies fixes, files deferred findings
---

# Forge Pre-Commit Review

This command is automatically invoked when `forge.sh` blocks a `git commit`.
Follow this protocol exactly.

## Context from Hook

The hook provides:

- **Signal file** — `.claude/agent-bus/signals/forge-{hash}.json`
- **Diff file** — temp file with `git diff --cached` output
- **Forge hash** — session identifier
- **Report file** — `.claude/logs/forge-{hash}.md`

$ARGUMENTS

## Orchestration Protocol

### Step 1: Read Signal File

```bash
cat .claude/agent-bus/signals/forge-{hash}.json
```

Extract: `maxRounds`, `autoDeferNits`, `diffFile`, `stagedFiles`.

### Step 2: Spawn Forge Reviewer (Round 1)

Spawn a `forge-reviewer` subagent using the Task tool:

```
subagent_type: forge-reviewer
prompt: |
  You are reviewing staged changes for a pre-commit Forge review.

  Signal file: {signal_file_path}
  Diff file: {diff_file_path}
  Forge hash: {forge_hash}
  Round: 1

  Read the diff file and the signal file, then:
  1. Delegate to codex (mcp__codex__codex) for a cross-model review
  2. Produce structured findings as a JSON array
  3. Log findings via forge-report.sh

  Return your findings JSON and your CONSENSUS/COUNTER/QUESTION signal.
```

### Step 3: Process Findings

Parse the reviewer's JSON findings. For each finding, apply the
**severity-action matrix**:

#### Severity-Action Matrix

| Severity   | Allowed actions        | Notes                                   |
| ---------- | ---------------------- | --------------------------------------- |
| `critical` | fix, defer             | Cannot dismiss — must fix or file issue |
| `major`    | fix, defer             | Cannot dismiss — must fix or file issue |
| `minor`    | fix, dismiss, defer    | Author decides                          |
| `nit`      | fix, dismiss, defer    | Auto-deferred if `autoDeferNits=true`   |

**Auto-defer nits:** If `autoDeferNits` is true, immediately mark all
nit-severity findings as deferred without negotiation. Log them in the report.

**Critical/major enforcement:** If the author attempts to dismiss a critical or
major finding, reject the dismissal and require fix or defer.

### Step 4: Respond Per Finding

For each non-auto-deferred finding, decide:

- **fix** — Edit the file to address the finding. Then re-stage:
  ```bash
  git add {file}
  ```
- **dismiss** — Provide reasoning why the finding is not applicable.
- **defer** — Mark for issue filing. Provide reasoning for deferral.

Build responses JSON:

```json
[
  { "findingId": "F-001", "action": "fix", "reasoning": "Fixed as suggested" },
  { "findingId": "F-002", "action": "dismiss", "reasoning": "False positive — this path is unreachable" },
  { "findingId": "F-003", "action": "defer", "reasoning": "Valid but out of scope for this commit" }
]
```

Log responses:

```bash
.claude/agent-bus/forge-report.sh {hash} responses {round} '{responses_json}'
```

### Step 5: Check Round Outcome

After responding:

- If all findings are resolved (fixed, dismissed-and-accepted, or deferred) →
  **CONSENSUS**
- If the reviewer countered any dismissal → continue to next round
- If max rounds reached → auto-defer all remaining unresolved findings

Log outcome:

```bash
.claude/agent-bus/forge-report.sh {hash} round-summary {round} "CONSENSUS"
```

### Step 6: Subsequent Rounds (if needed)

#### Scoped Re-review Rules

Rounds 2+ are **scoped** — the reviewer ONLY reviews changes made by fixes in
the previous round. This prevents infinite expansion.

1. Capture the new staged diff (which now includes your fixes):
   ```bash
   git diff --cached > {new_diff_file}
   ```
2. Spawn the forge-reviewer again with the **new** diff and round context
3. The reviewer MUST NOT introduce findings on unchanged code
4. The reviewer can only flag issues in lines that were modified by your fixes

#### Round Cap Enforcement

Track the current round against `maxRounds` from the signal file.

- **Before max round:** Normal negotiation — reviewer can COUNTER, author
  responds
- **At max round (final):** This is the last round. After this:
  - Any remaining COUNTER findings → auto-deferred to issues
  - No further negotiation permitted
  - Log: "Round cap reached — remaining findings deferred"

### Step 7: File Deferred Findings

Collect all deferred findings (explicit deferrals + auto-deferred nits +
round-cap deferrals).

Log them:

```bash
.claude/agent-bus/forge-report.sh {hash} deferred '{deferred_json}'
```

**Filing** via `forge-defer.sh`:

```bash
# File all deferred findings (auto-detects APS vs GitHub context)
FORGE_HASH={hash} FORGE_SOURCE="forge round {round}" \
  .claude/agent-bus/forge-defer.sh batch '{deferred_json}'
```

The utility handles:
- GitHub Issues with `forge:deferred` + `area:{category}` labels
- APS work items if branch/commit references an APS module
- Deduplication against existing open `forge:deferred` issues

### Step 8: Complete and Re-commit

1. Update the signal file status to `consensus` or `deferred`
2. Log completion:
   ```bash
   .claude/agent-bus/forge-report.sh {hash} complete "consensus" {total_rounds}
   ```
3. Re-run the original `git commit` command using `--no-verify` to bypass the
   forge hook, which would otherwise generate a new hash and run again

**Important:** The re-commit should use `--no-verify` to avoid re-triggering
the forge hook on the same changes. The forge review is already complete.

## Quick Reference

```
Read signal:    cat .claude/agent-bus/signals/forge-{hash}.json
Spawn reviewer: Task tool, subagent_type: forge-reviewer
Log findings:   .claude/agent-bus/forge-report.sh {hash} findings {round} '{json}'
Log responses:  .claude/agent-bus/forge-report.sh {hash} responses {round} '{json}'
Log deferred:   .claude/agent-bus/forge-report.sh {hash} deferred '{json}'
Log complete:   .claude/agent-bus/forge-report.sh {hash} complete {outcome} {rounds}
Re-stage:       git add {file}
Re-commit:      git commit --no-verify -m "{original message}"
```

## Error Handling

| Error                          | Action                                     |
| ------------------------------ | ------------------------------------------ |
| Reviewer fails to produce JSON | Treat as 0 findings, log error in report   |
| Codex delegation fails         | Continue with Claude-only review            |
| Signal file missing            | Abort forge, allow commit to proceed       |
| Diff file missing              | Abort forge, allow commit to proceed       |
| Round timeout                  | Auto-defer remaining, complete session     |
