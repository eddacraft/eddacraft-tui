---
name: council
description: |
  Unified local-first review router for Codex. Routes to this repo's Council
  workflow and reviewer personas without assuming bundled scripts or assets.
---

# Council

Local-first review router. Council is the canonical review interface - GitHub PRs are publication receipts, not the workspace.

## Canonical assets

Council is represented here as a Codex-facing router. The full reusable Council
assets live outside this `.codex/skills/council/` directory; do not assume
bundle-local `agents/`, `scripts/`, `prompts/`, or `references/` paths exist in
this repo checkout.

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

## Repo-local layout

This Codex skill directory contains only `SKILL.md` and `skill.meta.json`.
Resolve Council commands, reviewer assets, and session details through the
repo's authoritative Council documentation and available runtime tools.

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

Use the repo's available Council runtime or command surface to list/resume any
active session for the same target. This Codex router does not provide its own
session scripts.

### 2. Determine review target

| Context                      | Target type | Diff command           |
| ---------------------------- | ----------- | ---------------------- |
| Pre-commit / "review staged" | `staged`    | `git diff --cached`    |
| "Review this branch"         | `branch`    | `git diff main...HEAD` |
| "Review `<file>`"            | `files`     | `git diff -- <file>`   |
| Default (worktree changes)   | `worktree`  | `git diff`             |
| "Review `<commit>`"          | `commit`    | `git show <commit>`    |

### 3. Initialize the session

Initialise a streaming review with the `quick` reviewer pack against the chosen
target using the repo's Council command/runtime.

### 4. Gather review context

Read the diff and the surrounding code; pull recent commit messages for intent.

### 5. Run the reviewer

Run the configured `council-reviewer` with the diff + context. Capture findings
as structured records with severity, category, location, description, and
suggested fix.

### 6. Record findings

For each finding, record severity, category, file/line evidence, description,
suggestion, source reviewer, and current status in the active Council session.

### 7. Drive convergence

Present findings ordered by severity. For each, the user picks **fix** / **defer** / **waive** / **dismiss** (dismiss is disallowed for critical/major):

Record each resolution as `fixed`, `deferred`, `waived`, or `dismissed` with a
brief evidence-backed reason.

After fixes, offer a scoped re-review on just the changed lines.

### 8. Attach evidence

Attach the relevant test, lint, typecheck, docs, or manual-review evidence to
the finding or session.

### 9. Converge

Close or mark the session converged only after critical and major findings have
been fixed, explicitly waived, or moved into tracked follow-up work.

## Workflow: Batch Council

Broader, more formal review for milestone prep, release, or significant PRs.

| Aspect    | Streaming       | Batch                              |
| --------- | --------------- | ---------------------------------- |
| Pack      | `quick` (1)     | `standard` / `full` / `full:codex` |
| Scope     | Current changes | Branch vs base, full changeset     |
| Reviewers | 1               | 3-5 specialists                    |
| Output    | Convergence     | Formal summary bundle              |

### 1. Initialize with a broader pack

Initialise a batch review with the `standard`, `full`, or `full:codex` reviewer
pack against the branch diff from `main` using the repo's Council
command/runtime.

### 2. Dispatch reviewers in parallel

`standard` pack (3 reviewers):

| Role            | Agent type                   |
| --------------- | ---------------------------- |
| General quality | `council-reviewer` |
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

`full:codex` pack - dispatch the selected Codex reviewer roles through the
available runtime, passing the branch diff and repository context to each role.
Merge their structured findings into the active session by source role.

### 3. Supervise -> Debate -> Judge (the `/council-full` flow)

For the supervised pipeline, follow the repo's available Council command/runtime. The flow is:

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

Reviewer contracts and wire formats are documented by the active Council runtime
or the repo-level Council documentation, not by bundle-local files in this
Codex skill directory.

### 4. Converge + publish

Publish the converged session through the available Council runtime when the PR
or commit needs a review receipt. Keep human-readable handoff notes under
`plans/reviews/` when a file artefact is useful.

## Workflow: status / publish / escalate

Use the repo's Council command/runtime to inspect status, publish a receipt, or
escalate an active session.

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

Reviewers send critical alerts immediately through the documented
`agent-messaging` channel (direct JSON `send_input` or configured mailbox files
under `.codex/agent-bus/messages/*.jsonl`). Do not call `send-message.sh` or
`receive-messages.sh`; those helper scripts are not part of this repo.

## File output

When review output needs to land on disk for handoff, write under **`plans/reviews/`** in the consuming project - never the project root. Suggested naming:

- `plans/reviews/YYYY-MM-DD-<branch-or-topic>.md`
- `plans/reviews/post-merge/<branch-slug>.md`

Session state is persisted by the Council runtime available in this repo. File output to `plans/reviews/` is optional - for human reference only.
