<!--
APS Module: Deferred Finding Filing
====================================
Auto-filing deferred findings as GitHub issues or APS issues
with structured metadata and traceability.
See: plans/aps-rules.md
-->

# Deferred Finding Filing

| ID    | Owner  | Status |
| ----- | ------ | ------ |
| DEFER | @aneki | Ready  |

## Purpose

When a finding is deferred -- either by negotiation in Forge or by cycle cap in
Temper -- it must be filed as a trackable issue rather than silently dropped.
This module handles auto-filing to GitHub Issues (default) or APS work items
(when the change is tied to an active APS plan), with structured metadata
linking back to the source PR, file, severity, and reasoning.

## In Scope

- GitHub Issue filing with `forge:deferred` label, structured body template
- APS issue filing (work item addition to relevant module) when APS context
  detected
- Auto-detection of APS context from commit message or branch name
- Issue deduplication (don't file the same finding twice)
- Category-to-label mapping (e.g., `area:security`, `area:edge-case`)
- Batch filing (multiple deferred findings in one operation)

## Out of Scope

- Issue triage or prioritization workflow
- Issue assignment to specific developers
- Closing issues when findings are eventually addressed
- Custom issue templates beyond the standard format

## Interfaces

**Depends on:**

- FNEG module — provides deferred findings with metadata
- TEMPER module — provides deferred findings from CI cycle cap
  (Note: TEMPER also depends on DEFER — this is a mutual runtime
  dependency, not a build cycle. TEMPER calls DEFER to file
  remaining findings; DEFER accepts findings from TEMPER as input.
  Resolution order: implement DEFER first, then TEMPER calls it.)
- GitHub CLI (`gh`) — issue creation
- APS plan files — module specs for APS issue filing
- Git — branch name and commit message for APS context detection

> **Note:** DEFER and TEMPER have a mutual dependency: TEMPER calls DEFER to file
> findings, while DEFER lists TEMPER as an input source. This is not circular at
> runtime — DEFER is a library called by both Forge and Temper. DEFER was
> implemented first; TEMPER invokes it via shell commands.

**Exposes:**

- Filing utility — callable from both Forge (local) and Temper (CI)
- `forge:deferred` label — applied to all filed issues
- `area:<category>` labels — per-finding category labels
- Filing report — summary of issues filed per Forge/Temper session

## Constraints

- Deferred findings are never silently dropped -- filing must succeed or
  produce a visible error
- GitHub Issue filing requires `gh` CLI authenticated and available
- APS filing requires write access to `plans/modules/*.aps.md`
- Deduplication checks against existing open issues with `forge:deferred` label

## Ready Checklist

- [x] Purpose and scope are clear
- [x] Dependencies identified (FNEG, TEMPER, gh CLI, APS plans)
- [x] All tasks defined
- [x] Issue template defined in design doc

## Tasks

### DEFER-001: Implement GitHub Issue filing

- **Intent:** Deferred findings are filed as GitHub Issues with structured
  metadata and labels
- **Expected Outcome:** Each deferred finding creates a GH issue with title
  `[forge] <description>`, body containing source PR, file, line, severity,
  category, reviewer reasoning, and author deferral reasoning
- **Validation:** `gh issue list --label forge:deferred` returns issues matching
  deferred findings from a Forge session
- **Confidence:** high

### DEFER-002: Implement category-to-label mapping

- **Intent:** Filed issues are labeled with finding category for filtering and
  triage
- **Expected Outcome:** Issues receive both `forge:deferred` and
  `area:<category>` labels (e.g., `area:security`, `area:edge-case`,
  `area:performance`, `area:style`)
- **Validation:** Filed issues have both the `forge:deferred` label and the
  appropriate `area:*` label
- **Dependencies:** DEFER-001
- **Confidence:** high

### DEFER-003: Implement APS context detection and filing

- **Intent:** When the current work is tied to an APS plan, deferred findings
  are filed as APS work items instead of (or in addition to) GH issues
- **Expected Outcome:** If the commit message or branch name references an APS
  module (e.g., `FORGE-001`, `plans/modules/`), the finding is added as a Draft
  work item in the relevant module's task list
- **Validation:** A deferred finding from a branch named `feat/FORGE-001-hook`
  creates a new Draft task in the FORGE module
- **Dependencies:** DEFER-001
- **Confidence:** medium

### DEFER-004: Implement issue deduplication

- **Intent:** The same finding is not filed as multiple issues across Forge
  sessions
- **Expected Outcome:** Before filing, the utility checks for existing open
  issues with matching file, line range, and description. If a match exists,
  the existing issue is updated with a comment instead of creating a duplicate
- **Validation:** Running Forge twice on the same code with the same deferred
  finding does not create two issues
- **Dependencies:** DEFER-001
- **Confidence:** medium

### DEFER-005: Implement batch filing and filing report

- **Intent:** Multiple deferred findings from a single session are filed
  efficiently with a summary report
- **Expected Outcome:** All deferred findings from a Forge or Temper session are
  filed in one batch, and a summary (count, links, categories) is appended to
  the Forge report or Temper PR comment
- **Validation:** A session with 3 deferred findings produces 3 issues and a
  summary listing all 3 with links
- **Dependencies:** DEFER-001, DEFER-002
- **Confidence:** high
