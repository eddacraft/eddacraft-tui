# Forge & Temper: Autonomous Code Review Pipeline

**Date**: 2026-02-24 **Status**: Approved **Author**: aneki + Claude Code
(brainstorming session)

## Problem

The current review workflow has compounding friction:

1. Code is written by Claude Code, committed, and pushed.
2. CI reviewers (copilot, claude-review) post comments.
3. The user manually kicks `/addressing-pr-reviews` to fix them.
4. Fixes trigger another review cycle. New comments appear.
5. Repeat 5-7 times over hours before the PR is green.

Root causes:

- **No pre-commit review** — issues are caught after they hit GitHub, where each
  fix is expensive (commit, push, CI, re-review).
- **No cap on review cycles** — bots always find something, and each fix gives
  them fresh material.
- **Manual gates** — the user must trigger each addressing round.
- **No deferral mechanism** — every finding is treated as blocking, even nits.

## Solution

Two complementary phases with hard caps on iteration:

- **Forge** (pre-commit, local): Agent negotiation catches issues before they
  leave the machine.
- **Temper** (post-push, GitHub Actions): Self-healing loop auto-addresses CI
  review comments.

Both are independently toggleable. Deferred findings are tracked as GitHub
issues or APS issues, never silently dropped.

## Pipeline Overview

```
You: "implement feature X"
  |
  v
Phase 1: WRITE
  Claude Code writes the code (unchanged from today)
  |
  v
Phase 2: FORGE (pre-commit, local)
  Reviewer agent reviews the staged diff
  Author + reviewer negotiate (max 3 rounds)
  Per-finding outcomes: fix / dismiss / defer (-> GH or APS issue)
  Consensus reached -> commit -> push -> PR
  |
  v
Phase 3: TEMPER (post-push, GitHub Actions)
  CI review fires (claude-review / copilot / other models)
  Auto-address workflow triggers on review comments
  Max 2 self-heal cycles, then stops
  Non-blocking findings -> deferred to issues
  |
  v
Phase 4: HUMAN GATE
  CI green, all threads resolved or deferred
  User reviews the PR and merges (or requests changes)
```

## Phase 2: The Forge

### Existing Infrastructure (reused)

| Component                                     | Location                                                             | Role in Forge                      |
| --------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------- |
| `/negotiate` command + skill                  | `.claude/commands/negotiate.md`, `.claude/skills/agent-negotiation/` | Core negotiation loop              |
| `CONSENSUS/COUNTER/QUESTION` protocol         | All agent specs                                                      | Finding resolution protocol        |
| `send-message.sh` / `receive-messages.sh`     | `.claude/agent-bus/`                                                 | Structured finding exchange        |
| `/delegate` command (codex MCP)               | `.claude/commands/delegate.md`                                       | Cross-model review (gpt-5.2-high)  |
| `agent-bus/schema.json`                       | `.claude/agent-bus/schema.json`                                      | Finding message format             |
| `on-agent-stop.sh` + trigger protocol         | `.claude/hooks/on-agent-stop.sh`                                     | Chain from reviewer to fix cycle   |
| Agent specs (code-reviewer, security-analyst) | `.claude/agents/`                                                    | Already have negotiation protocols |

### New Components

| Component                                    | Purpose                                                                               |
| -------------------------------------------- | ------------------------------------------------------------------------------------- |
| `forge.sh` hook (PreToolUse on `git commit`) | Entry point — intercepts commit, launches negotiation                                 |
| `forge-reviewer` agent                       | Specialized diff reviewer (wraps code-reviewer + security-analyst + codex delegation) |
| Deferred finding filing logic                | Auto-files non-blocking findings as GH/APS issues                                     |
| `CLAUDE_FORGE_ENABLED` env var               | Toggle                                                                                |

### How It Works

1. `forge.sh` intercepts `git commit`, captures staged diff via
   `git diff --cached`.
2. Spawns a negotiation between the current session (author) and a
   `forge-reviewer` subagent.
3. `forge-reviewer` delegates to codex (gpt-5.2-high) for a cross-model review,
   then presents findings using the existing `finding` message type.
4. Negotiation runs using the existing protocol — `CONSENSUS` to agree,
   `COUNTER` to push back.
5. Fixes get applied and re-staged between rounds.
6. After the final round, unresolved findings get filed as GH issues (tagged
   `forge:deferred`) or APS issues.
7. Forge report saved to `.claude/logs/forge-{hash}.md`.
8. Commit proceeds.

### Negotiation Protocol

Each round:

```
Reviewer -> list of findings (structured JSON)
  Each finding: { file, line, severity, category, description, suggestion }
  Severity: critical | major | minor | nit

Author -> response per finding
  Each response: { action: "fix" | "dismiss" | "defer", reasoning }

  fix    -> author edits the file, re-stages
  dismiss -> reviewer either accepts or escalates (costs a round)
  defer   -> filed immediately as issue, removed from negotiation
```

### Round Behavior

- **Round 1**: Reviewer presents all findings. Author fixes criticals/majors,
  defers or dismisses the rest.
- **Round 2**: Reviewer re-reviews only the changed lines from fixes. New
  findings on fix code only. Author responds.
- **Round 3**: Final round. Any remaining disagreements get deferred to issues.
  No more negotiation.

### Rules

- Reviewer only reviews the **diff**, never the whole codebase (scoped review).
- Round 2+ only reviews **new changes from fixes**, not the original diff again.
- Criticals MUST be fixed (not dismissable) — security vulns, data loss,
  crashes.
- Nits are auto-deferred if not fixed in round 1 (never worth arguing about).
- The reviewer cannot introduce new findings on unchanged code in rounds 2-3.

### Finding Categories

| Category         | Examples                                            | Default action    |
| ---------------- | --------------------------------------------------- | ----------------- |
| Security         | Injection, secrets, auth bypass                     | Must fix          |
| Correctness      | Logic errors, off-by-one, null deref                | Must fix          |
| Edge cases       | Missing error handling, boundary conditions         | Author decides    |
| Performance      | Unnecessary allocations, O(n^2) where O(n) possible | Author decides    |
| Style/convention | Naming, formatting, pattern adherence               | Auto-defer as nit |
| Test coverage    | Missing tests for new code paths                    | Defer to issue    |

### The `forge-reviewer` Agent

A new agent (`.claude/agents/forge-reviewer.md`) that:

- Receives the staged diff as input.
- Delegates to codex MCP for a cross-model review (Claude wrote it, GPT reviews
  it).
- Structures findings using the existing agent-bus schema.
- Participates in negotiation using the existing `CONSENSUS/COUNTER/QUESTION`
  protocol.
- Knows the round cap — auto-defers remaining findings in the final round.

## Phase 3: The Temper

### Trigger Modes

| Mode       | Trigger                                  | Label required?                              |
| ---------- | ---------------------------------------- | -------------------------------------------- |
| **Auto**   | PR review comments posted                | Yes — `forge:tempered` label must be present |
| **Manual** | `workflow_dispatch` with PR number input | No — works on any PR                         |

The manual trigger works regardless of `CLAUDE_TEMPER_ENABLED` — that toggle
only controls the automatic trigger.

### Self-Healing Loop

```
PR review comments posted
  |
  v
temper.yml fires (if forge:tempered label present, or manual dispatch)
  |
  v
Cycle 1:
  - Fetch unresolved threads (GraphQL)
  - Categorize: fix / reply / defer
  - Critical + major -> fix and commit
  - Minor + nit -> defer to GH issue (tagged forge:deferred)
  - Questions -> reply with reasoning
  - Resolve all threads
  - Push, post summary
  |
  v
CI runs -> new review fires -> new comments?
  |
  +-- No new comments -> done, ready for human merge
  |
  v
Cycle 2 (final):
  - Same process but ONLY addresses findings on lines changed in cycle 1
  - Everything else -> deferred to GH issue
  - Push, post summary with "Temper complete -- remaining items deferred"
  |
  v
Done. No cycle 3. Human reviews and merges.
```

### Key Constraints

- **Max 2 cycles** — hard cap, enforced by a cycle counter in the workflow.
- **Cycle 2 is scoped** — only reviews changes from cycle 1 fixes, not the whole
  PR again.
- **Bot mentions avoided** — no `@copilot`, no `@coderabbitai` (prevents
  re-triggering).
- **Deferred findings always tracked** — GH issue with `forge:deferred` label,
  linking back to the PR.
- **The `forge:tempered` label** is auto-applied by the Forge when it creates a
  PR, or manually added.

### Differences from Current addressing-pr-reviews Skill

| Current skill                                       | Temper                                      |
| --------------------------------------------------- | ------------------------------------------- |
| Manual trigger (user runs `/addressing-pr-reviews`) | Auto-trigger on review comments             |
| Unlimited cycles (user keeps re-running)            | Hard cap at 2 cycles                        |
| All findings treated equally                        | Minor/nit auto-deferred after cycle 1       |
| No issue filing                                     | Deferred findings -> GH issues              |
| One workflow                                        | Two modes: auto (label) + manual (dispatch) |

The existing `/addressing-pr-reviews` skill remains available as a fully local
escape hatch.

## Configuration & Toggles

```bash
# Local (env vars)
CLAUDE_FORGE_ENABLED=true          # Pre-commit negotiation
CLAUDE_FORGE_MAX_ROUNDS=3          # Negotiation round cap
CLAUDE_FORGE_AUTO_DEFER_NITS=true  # Auto-defer nits without negotiating

# GitHub (repo-level Actions variables)
CLAUDE_TEMPER_ENABLED=true         # Auto self-healing on review comments
CLAUDE_TEMPER_MAX_CYCLES=2         # Self-healing cycle cap
```

| Scenario               | Forge | Temper | What happens                                    |
| ---------------------- | ----- | ------ | ----------------------------------------------- |
| Full autonomous        | on    | on     | Pre-commit review + auto self-healing post-push |
| Local review only      | on    | off    | Pre-commit review, manual PR handling           |
| Auto self-healing only | off   | on     | No pre-commit, but PR reviews auto-addressed    |
| Everything off         | off   | off    | Current manual workflow (unchanged)             |
| Manual temper only     | off   | off    | Use `workflow_dispatch` on any PR ad-hoc        |

## Deferred Findings -> Issues

When a finding is deferred (by negotiation in Forge or by cycle cap in Temper):

**GitHub Issue** (default):

```
Title: [forge] Missing error boundary in UserProfile component
Labels: forge:deferred, area:<category>
Body:
  Source: PR #354, forge round 2 / temper cycle 1
  File: src/components/UserProfile.tsx:42
  Severity: minor
  Category: edge-case
  Description: <finding description>
  Reviewer reasoning: <why this matters>
  Author reasoning for deferral: <why not now>
```

**APS Issue** (if the PR is tied to an active APS plan):

- Adds a work item to the relevant module's issue log.
- Links back to the PR.
- Tagged with severity for prioritization.

The choice is automatic: if the commit message or branch name references an APS
plan/module, findings go to APS. Otherwise, GH issues.

## Files to Create

| File                               | Purpose                                                         |
| ---------------------------------- | --------------------------------------------------------------- |
| `.claude/hooks/forge.sh`           | PreToolUse hook — intercepts `git commit`, launches negotiation |
| `.claude/agents/forge-reviewer.md` | Diff reviewer agent with codex delegation                       |
| `.github/workflows/temper.yml`     | Self-healing workflow (auto + manual dispatch)                  |
| `.claude/skills/forge/SKILL.md`    | Forge skill documentation                                       |

## Files to Modify

| File                             | Change                                                         |
| -------------------------------- | -------------------------------------------------------------- |
| `.claude/settings.json`          | Add `CLAUDE_FORGE_ENABLED` default, hook registration          |
| `.claude/hooks/on-agent-stop.sh` | Ensure forge-reviewer triggers are handled                     |
| `.claude/agent-bus/schema.json`  | Add `forge-finding` message subtype if needed                  |
| `CLAUDE.md`                      | Document Forge/Temper in hook behavior table and env var table |

## Success Criteria

1. Writing code + committing triggers the Forge review automatically.
2. Forge negotiation completes within 3 rounds — no infinite loops.
3. Non-blocking findings are filed as GH/APS issues, never silently dropped.
4. PR creation triggers CI review; Temper auto-addresses comments within 2
   cycles.
5. Human only needs to review the final PR and click merge.
6. Everything is toggleable — Forge and Temper can be independently
   enabled/disabled.
7. Existing `/addressing-pr-reviews` skill continues to work as a manual
   fallback.

## Non-Goals

- Replacing human judgment on architectural decisions.
- Auto-merging PRs (human always gates the final merge).
- Reviewing entire codebases (Forge is scoped to the staged diff only).
- Replacing CI checks (lint, type-check, tests still run normally).
