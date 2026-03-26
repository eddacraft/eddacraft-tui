<!--
APS Module: Forge Negotiation Protocol
=======================================
Structured finding/response protocol with round management, severity-based
actions, scoped re-review, and auto-defer behavior.
See: plans/aps-rules.md
-->

# Forge Negotiation Protocol

| ID   | Owner  | Status      |
| ---- | ------ | ----------- |
| FNEG | @aneki | Complete |

## Purpose

Define and implement the structured negotiation protocol between the author
(current session) and the forge-reviewer agent. Each finding gets a per-finding
response (fix/dismiss/defer), rounds are capped at 3, and subsequent rounds
only review changes from fixes -- never the original diff again. Nits are
auto-deferred if not fixed in round 1.

## In Scope

- Finding schema: `{ file, line, severity, category, description, suggestion }`
- Response schema: `{ action: "fix" | "dismiss" | "defer", reasoning }`
- Severity levels: critical, major, minor, nit
- Finding categories: security, correctness, edge-case, performance,
  style/convention, test-coverage
- Round cap enforcement (default 3, configurable via `CLAUDE_FORGE_MAX_ROUNDS`)
- Scoped re-review: rounds 2+ only review new changes from fixes
- Critical findings cannot be dismissed (must fix or defer to issue)
- Nit auto-defer: nits not fixed in round 1 are auto-deferred
- Fix-and-restage flow: author fixes, re-stages, round continues

## Out of Scope

- The hook and agent scaffolding (Module 1: FORGE)
- Issue filing for deferred findings (Module 3: DEFER)
- Temper workflow (Module 4: TEMPER)
- The existing `/negotiate` command itself (already implemented)

## Interfaces

**Depends on:**

- FORGE module — hook and agent that initiate negotiation
- Agent bus schema — `schema.json` message format
- `/negotiate` command — orchestration of rounds

**Exposes:**

- Finding schema extension — `forge-finding` fields on agent-bus messages
- Round behavior rules — documented in skill, enforced in negotiation
- Severity-action matrix — which actions are allowed per severity level
- `CLAUDE_FORGE_MAX_ROUNDS` — round cap configuration
- `CLAUDE_FORGE_AUTO_DEFER_NITS` — nit auto-defer toggle

## Constraints

- Round 2+ must not introduce new findings on unchanged code
- Critical findings (security, data loss, crashes) cannot be dismissed
- Each round must produce a clear resolution for every active finding
- Auto-deferred nits must still be filed as issues (handled by DEFER module)

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (FORGE, agent-bus, /negotiate)
- [x] All tasks defined
- [x] Severity-action matrix defined in design doc

## Tasks

### FNEG-001: Extend agent-bus schema with finding fields

- **Status:** Complete
- **Intent:** Add forge-specific finding and response fields to the agent-bus
  message schema so structured findings flow through the existing messaging system
- **Expected Outcome:** `schema.json` supports `forge-finding` message subtype
  with severity, category, file, line, description, and suggestion fields
- **Validation:** Sample forge finding messages validate against the updated
  schema
- **Files:** `.claude/agent-bus/schema.json`
- **Confidence:** high
- **Notes:** Added `forgeFinding`, `forgeResponse`, and `forgeSignal` definitions
  to schema.json. Finding fields: id, file, line, severity, category, description,
  suggestion, codexAgreed, status. Response fields: findingId, action, reasoning.

### FNEG-002: Implement round cap enforcement

- **Status:** Complete
- **Intent:** Negotiation terminates after the configured maximum rounds, with
  all remaining findings auto-deferred
- **Expected Outcome:** After `CLAUDE_FORGE_MAX_ROUNDS` rounds, negotiation stops
  and unresolved findings are marked for deferral
- **Validation:** A negotiation with unresolved findings after round 3 terminates
  with deferred findings listed
- **Dependencies:** FNEG-001
- **Confidence:** high

### FNEG-003: Implement scoped re-review for rounds 2+

- **Status:** Complete
- **Intent:** Subsequent rounds only review lines changed by fixes, not the
  original diff
- **Expected Outcome:** The forge-reviewer receives only the new changes (diff of
  fixes) in rounds 2 and 3, and cannot introduce findings on unchanged code
- **Validation:** A round-2 review that attempts to flag unchanged code produces
  no new findings on those lines
- **Dependencies:** FNEG-001
- **Confidence:** medium

### FNEG-004: Implement severity-action matrix

- **Status:** Complete
- **Intent:** Enforce which response actions are allowed per finding severity
  level
- **Expected Outcome:** Criticals cannot be dismissed (only fix or defer to
  issue). Nits not fixed in round 1 are auto-deferred when
  `CLAUDE_FORGE_AUTO_DEFER_NITS=true`
- **Validation:** Attempting to dismiss a critical finding is rejected; nits
  surviving round 1 are automatically marked as deferred
- **Dependencies:** FNEG-001
- **Confidence:** high

### FNEG-005: Implement fix-and-restage flow

- **Status:** Complete
- **Intent:** When the author fixes a finding, the file is re-staged before the
  next round begins
- **Expected Outcome:** After a "fix" response, the author's changes are applied
  to the working tree and staged via `git add`, and the next round reviews
  only the new changes
- **Validation:** A fixed file appears in the staged diff for the subsequent
  round's scoped re-review
- **Dependencies:** FNEG-003, FNEG-004
- **Confidence:** medium
