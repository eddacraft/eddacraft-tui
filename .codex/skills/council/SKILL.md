---
name: council
description: |
  Unified local-first review skill. Bundles reviewer agents, session scripts,
  data schemas, and Codex prompts. Runs Streaming Council during implementation
  and Batch Council at milestones; treats GitHub PRs as publication artefacts,
  not the primary review workspace.
---

# Council

Self-contained local-first review system. Council is the canonical review interface - GitHub PRs are publication receipts, not the workspace.

## Canonical assets

Council is a bundle of neutral catalogue assets. The canonical reusable agent
definitions live under top-level `agents/`; this skill may also carry packaged
copies under `agents/` for runtimes that install a skill as a self-contained
plugin. `code-env` owns emitting the selected assets into target-specific
locations such as `.claude/agents/` or `.opencode/agents/`.

Council staff agents:

- `council-supervisor`
- `council-debate`
- `council-judge`

Council reviewer pool:

- `council-reviewer`
- `adversarial-reviewer`
- `security-analyst`
- `operations-reviewer`
- `pragmatic-lead`
- `kernel-maintainer`

## Packaged bundle layout

```
council/
|-- SKILL.md                  <- this file (entrypoint)
|-- agents/                   <- reviewer + orchestrator personas
|   |-- council-reviewer.md
|   |-- council-supervisor.md
|   |-- council-debate.md
|   `-- council-judge.md
|-- scripts/                  <- session / finding / evidence / publish CRUD
|   |-- council-session.sh
|   |-- council-finding.sh
|   |-- council-evidence.sh
|   |-- council-publish.sh
|   `-- council-codex-reviewer.sh
|-- references/
|   |-- schema.md             <- human-readable data model
|   `-- schema.json           <- JSON Schema (draft-07)
|-- prompts/                  <- Codex (cross-model) reviewer prompts
|   |-- council-reviewer-codex.md
|   |-- security-codex.md
|   |-- adversarial-codex.md
|   |-- operations-codex.md
|   `-- pragmatic-lead-codex.md
`-- commands/
    `-- council-full.md       <- /council-full slash command
```

## Trigger

Activate when the user says: `council`, `council streaming`, `council batch`, `council status`, `council publish`, `council escalate`, or any variant like "review this", "run council", "local review".

## Resolving paths

Inside this skill, treat the bundle root as `${SKILL_DIR}`. The runtime resolves to the skill directory in the consuming project. All paths below are bundle-relative.

## Argument parsing

```
council                      -> streaming review of current worktree changes
council streaming            -> explicit streaming review
council batch                -> formal batch review
council status               -> show current session status
council publish              -> generate publication summary
council escalate             -> upgrade current session's reviewer pack
council <file/folder>        -> streaming review scoped to specific path(s)
council full                 -> batch review with all 5 reviewers
council full:codex           -> full cross-model Codex review (all 5 roles)
```

## Workflow: Streaming Council

Streaming Council is the default - low-latency review during implementation.

### 1. Check for an existing session

```bash
bash ${SKILL_DIR}/scripts/council-session.sh list --active
```

If an active session exists for the same target, resume it:

```bash
bash ${SKILL_DIR}/scripts/council-session.sh resume <session-id>
```

### 2. Determine review target

| Context                      | Target type | Diff command           |
| ---------------------------- | ----------- | ---------------------- |
| Pre-commit / "review staged" | `staged`    | `git diff --cached`    |
| "Review this branch"         | `branch`    | `git diff main...HEAD` |
| "Review `<file>`"            | `files`     | `git diff -- <file>`   |
| Default (worktree changes)   | `worktree`  | `git diff`             |
| "Review `<commit>`"          | `commit`    | `git show <commit>`    |

### 3. Initialize the session

```bash
SESSION_ID=$(bash ${SKILL_DIR}/scripts/council-session.sh init \
  --mode streaming \
  --target <type> \
  --pack quick \
  [--branch <name>] [--base <name>] [--files <f1,f2>])
```

### 4. Gather review context

Read the diff and the surrounding code; pull recent commit messages for intent.

### 5. Run the reviewer

Spawn the bundled reviewer agent (`agents/council-reviewer.md`) with the diff + context. The reviewer returns one JSON object: `{ findings: [...], summary: "..." }`.

### 6. Record findings

For each finding:

```bash
bash ${SKILL_DIR}/scripts/council-finding.sh add "$SESSION_ID" \
  --severity <critical|major|minor|nit> \
  --category <security|correctness|edge-case|performance|architecture|style|test-coverage|documentation> \
  --description "..." --file <path> --line <n> \
  --suggestion "..." --source council-reviewer
```

### 7. Drive convergence

Present findings ordered by severity. For each, the user picks **fix** / **defer** / **waive** / **dismiss** (dismiss is disallowed for critical/major):

```bash
bash ${SKILL_DIR}/scripts/council-finding.sh resolve "$SESSION_ID" <finding-id> \
  --status <fixed|deferred|waived|dismissed> --resolution "..."
```

After fixes, offer a scoped re-review on just the changed lines.

### 8. Attach evidence

```bash
bash ${SKILL_DIR}/scripts/council-evidence.sh run "$SESSION_ID" \
  --command "<test/lint cmd>" --description "..." --finding <finding-id>
```

### 9. Converge

```bash
bash ${SKILL_DIR}/scripts/council-session.sh close "$SESSION_ID" --status converged
```

## Workflow: Batch Council

Broader, more formal review for milestone prep, release, or significant PRs.

| Aspect    | Streaming       | Batch                              |
| --------- | --------------- | ---------------------------------- |
| Pack      | `quick` (1)     | `standard` / `full` / `full:codex` |
| Scope     | Current changes | Branch vs base, full changeset     |
| Reviewers | 1               | 3-5 specialists                    |
| Output    | Convergence     | Formal summary bundle              |

### 1. Initialize with a broader pack

```bash
SESSION_ID=$(bash ${SKILL_DIR}/scripts/council-session.sh init \
  --mode batch --target branch --pack standard --base main)
```

### 2. Dispatch reviewers in parallel

`standard` pack (3 reviewers):

| Role            | Agent type                   |
| --------------- | ---------------------------- |
| General quality | `council-reviewer` (bundled) |
| Security        | `security-analyst`           |
| Adversarial     | `adversarial-reviewer`       |

`full` pack (5 reviewers - all in parallel):

| Role            | Agent type             |
| --------------- | ---------------------- |
| General quality | `council-reviewer`     |
| Security        | `security-analyst`     |
| Adversarial     | `adversarial-reviewer` |
| Operations      | `operations-reviewer`  |
| Pragmatic lead  | `pragmatic-lead`       |

`kernel-maintainer` is part of the canonical reviewer pool and should be used
for stricter correctness, simplicity, performance, and dependency review when a
project or Council profile selects that reviewer.

`full:codex` pack - dispatch the 5 Codex roles via the bundled script:

```bash
DIFF_FILE=$(mktemp /tmp/council-diff-XXXXXX.patch)
git diff main...HEAD > "$DIFF_FILE"

for role in adversarial-codex security-codex council-reviewer-codex \
            operations-codex pragmatic-lead-codex; do
  bash ${SKILL_DIR}/scripts/council-codex-reviewer.sh \
    --role "$role" --diff "$DIFF_FILE" \
    --session-id "$SESSION_ID" \
    --prompt "${SKILL_DIR}/prompts/${role}.md" &
done
wait
```

Each script prints a JSON findings file path. Parse and merge into the session via `council-finding.sh add --source <role>`.

### 3. Supervise -> Debate -> Judge (the `/council-full` flow)

For the supervised pipeline, follow the slash command at `commands/council-full.md`. The flow is:

```
[parallel] All reviewers
   |
   v
[per output] council-supervisor   ->  REJECTED retries (max 2)
   |
   v
[contradictions?] council-debate  ->  binding verdict
   |
   v
council-judge                     ->  gate decision (BLOCK | WARN | PASS)
   |
   v
Action list (must_fix / should_fix / consider)
```

Agents and their contracts are documented in `agents/` and the wire formats in `references/schema.md`.

### 4. Converge + publish

```bash
bash ${SKILL_DIR}/scripts/council-publish.sh "$SESSION_ID" --pr      # PR body
bash ${SKILL_DIR}/scripts/council-publish.sh "$SESSION_ID" --commit  # Trailer
bash ${SKILL_DIR}/scripts/council-publish.sh "$SESSION_ID" --format json
```

## Workflow: status / publish / escalate

```bash
bash ${SKILL_DIR}/scripts/council-session.sh status [<session-id>]
bash ${SKILL_DIR}/scripts/council-publish.sh <session-id> --pr
```

**Escalate** an active session: re-init with a broader pack, dispatch the additional reviewers, merge new findings into the existing session ID.

## Reviewer packs

| Pack         | Reviewers                                 | When                              |
| ------------ | ----------------------------------------- | --------------------------------- |
| `quick`      | council-reviewer                          | Implementation-time, small change |
| `standard`   | council-reviewer + security + adversarial | Significant changes, PR prep      |
| `full`       | All 5 roles                               | Release review, high-risk         |
| `full:codex` | All 5 Codex roles                         | Cross-model second opinion        |

## Principles

1. **Council is canonical** - review state lives locally, not in PR threads.
2. **PRs are receipts** - they summarize reviewed work.
3. **Convergence over commentary** - drive findings to resolution; don't accumulate noise.
4. **Hooks are policy, not product** - Council works without hooks.
5. **Kindling augments, not anchors** - works without Kindling.

## Agent messaging

Council coordinates findings between reviewers via the `agent-messaging` sub-skill.

Reviewers send critical alerts immediately (don't wait for session end):

```bash
./send-message.sh --from security-analyst --to council-reviewer \
  --type alert --priority critical \
  --payload '{"vulnerability":"SQL injection","file":"src/db.ts","line":42}'
```

Orchestrator collects findings before synthesising a verdict:

```bash
./receive-messages.sh council-reviewer --format summary
```

## File output

When review output needs to land on disk for handoff, write under **`plans/reviews/`** in the consuming project - never the project root. Suggested naming:

- `plans/reviews/YYYY-MM-DD-<branch-or-topic>.md`
- `plans/reviews/post-merge/<branch-slug>.md`

Session state is always persisted via `scripts/council-finding.sh` and `scripts/council-session.sh`. File output to `plans/reviews/` is optional - for human reference only.
