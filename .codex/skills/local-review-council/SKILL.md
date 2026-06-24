---
name: local-review-council
description: Run Streaming Council, the local-first iterative mode of the Council review system.
---

# Local Review Council Skill

## Overview

`local-review-council` is the explicit entrypoint for **Streaming Council**, the default local-first mode of the Council review system.

Streaming Council is designed for iterative development inside the active workspace. It triggers a specialist reviewer (usually `code-reviewer`) to inspect the current changes in full project context and provide immediate feedback to the implementation agent or user. Its goal is not to simulate async PR comments, but to drive local convergence before publication.

This differs from Batch Council, which uses multiple personas to produce formal deliverables at milestone points. Streaming Council is the fast loop; Batch Council is the formal dossier.

**Review is an explicit workflow, not a commit side effect.** Start a review session at any point — you do not need to commit first.

## When to Apply

- After a significant set of file changes
- Before committing code (but not triggered by committing)
- When you want an "internal second opinion" on a technical decision
- To resolve tradeoffs between architecture, security, and performance
- When you want the PR to be the record of review rather than the workspace for review

## Session Commands

Council review operates through persistent sessions under `.claude/council/sessions/`.

### Start a session

```bash
bash .claude/council/council-session.sh init --target workspace
bash .claude/council/council-session.sh init --target branch --base main
bash .claude/council/council-session.sh init --target diff
```

Targets:

- **workspace** (default) — review all staged and unstaged changes
- **branch** — review the current branch against `--base`
- **diff** — review only staged changes

### Check session status

```bash
bash .claude/council/council-session.sh status              # latest session
bash .claude/council/council-session.sh status <session-id>  # specific session
bash .claude/council/council-session.sh list                 # all sessions
```

### Record and update findings

```bash
bash .claude/council/council-finding.sh add <session-id> \
  --severity critical --title "Missing validation" --file src/api.ts --evidence "..."

bash .claude/council/council-finding.sh resolve <session-id> f-001 --status fixed
```

Finding statuses: `open`, `fixed`, `deferred`, `waived`, `dismissed`

### Record events

```bash
bash .claude/council/council-session.sh add-event <session-id> \
  --type fix-applied --detail "Added input sanitization to handler"
```

### Close and publish

```bash
bash .claude/council/council-session.sh close <session-id>
bash .claude/council/council-publish.sh <session-id> --format summary
bash .claude/council/council-publish.sh latest --format pr-body
```

Formats: `summary` (full report), `pr-body` (concise PR description), `review-comment` (PR comment)

## Workflow

### Step 1: Initialize a Session

Start a review session against the current workspace, branch, or diff. The session is created locally — no commit or push required.

```
SESSION_ID=$(bash .claude/council/council-session.sh init --target workspace)
```

### Step 2: Trigger the Reviewer

Spawn the `code-reviewer` agent (or another specialist) with workspace context. The reviewer inspects changes and provides findings.

**Prompt for Reviewer:**

```markdown
Review the current staged and unstaged changes in the workspace.
Provide actionable feedback for the implementation agent.
If an issue is straightforward, use TRIGGER:implement-fix.
If an issue requires a tradeoff, use TRIGGER:negotiate.
Record each finding with severity and file references.
```

The reviewer should not act like a GitHub commenter. It should behave like a collaborator in a local review loop: identify blocking issues, suggest direct fixes, and escalate only the questions that need explicit judgement.

### Step 3: Record Findings

As findings emerge from the reviewer, record them in the session:

```bash
bash .claude/council/council-finding.sh add "$SESSION_ID" \
  --severity major --title "Unhandled error in auth flow" \
  --file "src/auth/handler.ts" \
  --evidence "catch block swallows error silently"
```

### Step 4: Parse Triggers and Iterate

Process reviewer output:

- **`TRIGGER:implement-fix`**: Apply the fix, then mark the finding as `fixed`.
- **`TRIGGER:negotiate`**: Start a formal negotiation using the `agent-negotiation` skill.

After each fix, re-review changed files. The core loop:

1. inspect
2. raise findings
3. patch or negotiate
4. re-review
5. converge

### Step 5: Converge and Publish

When all CRITICAL and MAJOR findings are addressed:

```bash
bash .claude/council/council-session.sh close "$SESSION_ID"
bash .claude/council/council-publish.sh "$SESSION_ID" --format pr-body
```

The publication output feeds into PR descriptions or review comments without rereading the diff.

## Council Personas

Instead of a single `code-reviewer`, the local council can pull in specialized perspectives to resolve complex technical decisions iteratively.

| Persona        | Agent                  | Focus                                            |
| -------------- | ---------------------- | ------------------------------------------------ |
| **Lead**       | `code-reviewer`        | Quality, simplicity, operations (multi-persona). |
| **Architect**  | `architect`            | High-level structure, boundaries, and contracts. |
| **Pragmatist** | `pragmatic-lead`       | Velocity, shipping, and practical tradeoffs.     |
| **Adversary**  | `adversarial-reviewer` | Security, edge cases, and breaking assumptions.  |
| **Security**   | `security-analyst`     | Threat modeling, vulnerability assessment.       |

### Cross-Persona Negotiation

If the maintainer rejects a change that the implementer believes is necessary for speed, the `pragmatic-lead` or `code-reviewer` can facilitate a negotiation.

```markdown
TRIGGER:negotiate:pragmatic-lead:!Discuss maintainer's concerns vs implementation speed for src/plugins/
```

The `code-reviewer` (as Lead) synthesizes different perspectives into a final decision.

## Session State

This repo's Claude Council stores session state as JSON files under `.claude/council/sessions/`:

| File pattern | Contents |
| ------------ | -------- |
| `*.json` | Session metadata, reviewer output, findings, events, and verdict state |

Session state survives multiple fix-and-re-review rounds. It is local to the repo and should not be committed.

## Comparison with Ceremonial Review

| Feature            | Batch Council              | Streaming Council                    |
| ------------------ | -------------------------- | ------------------------------------ |
| **Feedback Loop**  | Minutes/Hours              | Seconds/Minutes                      |
| **Context**        | Batch snapshot / personas  | Live workspace / implementation loop |
| **Communication**  | Formal synthesis           | Immediate dialogue                   |
| **Goal**           | Milestone scrutiny         | Direct local convergence             |
| **Final Artefact** | Deliverables and PR inputs | PR-ready session record              |

## Relationship to Council Gate

Streaming Council is the review engine. A commit or publish gate (Forge) may require it, but the gate is optional policy, not the definition of review itself.

If Council Gate is enabled, it invokes or verifies Streaming Council state before allowing the next action. If no gate is enabled, Streaming Council is still usable as an explicit developer workflow — which is the recommended default.

Commit hooks are guardrails, not the primary review interface. Use this skill to start reviews explicitly.

## Agent Messaging

This skill uses **agent-messaging** to share findings with other agents and receive context before analysis.

**REQUIRED SUB-SKILL:** Use `agent-messaging` for inter-agent communication.

Before starting analysis, check `agent-messaging` for its supported transport. In this repo that means direct JSON `send_input` handoffs and, when configured, mailbox files under `.codex/agent-bus/messages/*.jsonl`; do not assume shell helper scripts exist.

After completing analysis, send significant findings through that documented channel with structured JSON containing the file, issue, severity, and suggested fix.
