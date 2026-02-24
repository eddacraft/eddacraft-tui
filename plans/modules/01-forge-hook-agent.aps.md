<!--
APS Module: Forge Hook & Agent
==============================
Pre-commit hook and reviewer agent for the Forge phase of the
autonomous code review pipeline.
See: plans/aps-rules.md
-->

# Forge Hook & Agent

| ID    | Owner  | Status |
| ----- | ------ | ------ |
| FORGE | @aneki | Ready  |

## Purpose

Intercept `git commit` via a PreToolUse hook, launch a `forge-reviewer` subagent
that reviews the staged diff using cross-model delegation (codex/GPT), and
produce structured findings that enter the negotiation protocol. This is the
entry point for Phase 2 (Forge) of the autonomous code review pipeline.

## In Scope

- `forge.sh` PreToolUse hook that intercepts `git commit` commands
- `forge-reviewer` agent spec with codex delegation and finding output
- Integration with existing `/negotiate` command and agent-negotiation skill
- `CLAUDE_FORGE_ENABLED` env var toggle (default: false)
- Forge report logging to `.claude/logs/forge-{hash}.md`
- `forge:tempered` label auto-application on PR creation after Forge pass

## Out of Scope

- The negotiation protocol itself (Module 2: FNEG)
- Deferred finding filing logic (Module 3: DEFER)
- GitHub Actions workflow (Module 4: TEMPER)
- Configuration and documentation updates (Module 5: FTCFG)

## Interfaces

**Depends on:**

- Agent negotiation system — `/negotiate` command, `agent-negotiation` skill
- Agent bus — `send-message.sh`, `receive-messages.sh`, `schema.json`
- Codex MCP — cross-model delegation for review
- Existing agent specs — `code-reviewer.md`, `security-analyst.md` patterns

**Exposes:**

- `.claude/hooks/forge.sh` — PreToolUse hook entry point
- `.claude/agents/forge-reviewer.md` — reviewer agent spec
- `.claude/skills/forge/SKILL.md` — Forge skill documentation
- Forge report — `.claude/logs/forge-{hash}.md`

## Constraints

- Hook must exit cleanly and fast when `CLAUDE_FORGE_ENABLED` is not `true`
- Hook only fires on `git commit` tool use, not other Bash commands
- The forge-reviewer must review only the staged diff, never the full codebase
- Forge report must be written regardless of whether findings were produced

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (negotiation, agent-bus, codex MCP)
- [x] All tasks defined
- [x] Design doc approved (docs/plans/2026-02-24-forge-temper-review-pipeline.md)

## Tasks

### FORGE-001: Create forge.sh PreToolUse hook

- **Intent:** Intercept git commit commands and launch the Forge review pipeline
- **Expected Outcome:** Hook detects `git commit` in PreToolUse, captures staged
  diff, and initiates negotiation between the current session and forge-reviewer
- **Validation:** With `CLAUDE_FORGE_ENABLED=true`, a `git commit` triggers the
  forge-reviewer agent before the commit proceeds
- **Files:** `.claude/hooks/forge.sh`
- **Confidence:** high

### FORGE-002: Create forge-reviewer agent spec

- **Intent:** Define a specialized diff reviewer agent that delegates to codex
  for cross-model review and outputs structured findings
- **Expected Outcome:** Agent receives staged diff, delegates to codex MCP,
  structures findings using agent-bus schema, participates in negotiation protocol
- **Validation:** `forge-reviewer` agent produces valid JSON findings when given a
  diff input and responds to CONSENSUS/COUNTER signals
- **Files:** `.claude/agents/forge-reviewer.md`
- **Dependencies:** FORGE-001
- **Confidence:** high

### FORGE-003: Create Forge skill documentation

- **Intent:** Document the Forge workflow for agents and users
- **Expected Outcome:** Skill file explains the pre-commit review flow, round
  behavior, finding categories, and toggle behavior
- **Validation:** `.claude/skills/forge/SKILL.md` exists and covers the full
  Forge lifecycle
- **Files:** `.claude/skills/forge/SKILL.md`
- **Confidence:** high

### FORGE-004: Implement Forge report logging

- **Intent:** Persist a summary of each Forge session for auditability
- **Expected Outcome:** After negotiation completes, a markdown report is written
  with findings, responses, outcomes, and timing
- **Validation:** After a Forge run, `.claude/logs/forge-*.md` contains the
  session summary with per-finding outcomes
- **Dependencies:** FORGE-001, FORGE-002
- **Confidence:** high

### FORGE-005: Integration test for Forge pipeline

- **Intent:** Verify the end-to-end flow from git commit interception through
  negotiation to commit proceeding
- **Expected Outcome:** A test scenario demonstrates: hook fires, reviewer
  produces findings, negotiation resolves, commit proceeds
- **Validation:** Manual walkthrough with `CLAUDE_FORGE_ENABLED=true` completes
  without errors and produces a Forge report
- **Dependencies:** FORGE-001, FORGE-002, FORGE-003, FORGE-004
- **Confidence:** medium
