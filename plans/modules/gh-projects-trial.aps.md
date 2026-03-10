# GitHub Projects Trial

| ID | Owner | Status | Progress |
|----|-------|--------|----------|
| GHP | @aneki | Ready | 0/8 |

## Purpose

Run a 2-week trial of the dual-primary model described in
[2026-03-09-aps-vs-gh-projects-trial-decision-space.md](../../docs/plans/2026-03-09-aps-vs-gh-projects-trial-decision-space.md)
(PR #525). APS remains canonical for planning semantics; GitHub Projects becomes
canonical for execution telemetry. This module tracks the infrastructure setup,
backfill of outstanding work items, reconciliation cadence, and the final
decision gate.

## In Scope

- GitHub Project board creation and field configuration
- Built-in project automation (status transitions on PR events)
- Backfilling GH issues for all in-progress and outstanding APS work items
- Issue/PR template updates for APS↔GH linking contract
- Twice-weekly reconciliation checkpoints
- End-of-trial retrospective and decision gate
- Expansion to next feature wave if trial succeeds

## Out of Scope

- Custom CLI tooling or scripts for APS↔GH sync (evaluate post-trial)
- Changing APS plan format or semantics
- Multi-user/team workflow changes (single-developer trial)

## Interfaces

**Depends on:**

- PR #525 — Decision space document and template updates (must merge first)
- `plans/reviews/next.md` — Source of outstanding work items for backfill
- `plans/modules/*.aps.md` — APS modules with in-progress tasks

**Exposes:**

- GitHub Project board with APS-linked issues for all active work
- Updated issue/PR templates with APS context fields
- Reconciliation runbook for ongoing APS↔GH drift checks
- Trial retrospective with data for adopt/adjust/abandon decision

## Decisions

**D-GHP-001:** Automation approach

- **Options:** (a) GitHub built-in project workflows only, (b) GitHub Actions
  for custom sync, (c) Claude Code hook for auto-creating issues
- **Resolution:** Option (a) — Built-in project workflows. Keep the trial
  lightweight. Evaluate custom tooling only if manual overhead proves too high.
- **Status:** Resolved

**D-GHP-002:** Backfill scope

- **Options:** (a) Only in-progress APS tasks, (b) All outstanding work
  including upcoming modules, (c) Incremental — start with in-progress, add more
  as collated
- **Resolution:** Option (c) — Incremental. Backfill current in-progress items
  immediately (MAINT-005–008, STACK-018–019, open PRs). Additional items added
  as @aneki finishes collating remaining work.
- **Status:** Resolved

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| APS↔GH drift during trial | medium | Twice-weekly reconciliation checkpoints (GHP-004) |
| Overhead of maintaining two systems feels too high | medium | Decision gate (GHP-008) provides explicit off-ramp |
| Backfilled issues become stale if not actively worked | low | Only backfill items that are actively in-progress or ready |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified (PR #525)
- [x] Tasks defined
- [x] Decisions resolved (D-GHP-001, D-GHP-002)

## Waves

**Wave 1** — Trial infrastructure (GHP-001 through GHP-004). Sets up the board,
automation, initial backfill, and first reconciliation checkpoint.

**Wave 2** — Evaluation and decision (GHP-005 through GHP-008). Retrospective,
additional backfill, reconciliation runbook, and the decision gate. GHP-005–007
feed into GHP-008; the decision gate is the final task in Wave 2.

**Wave 3** — Operational rollout. Begins only after GHP-008 decides to adopt or
adjust. Open for additional tasks as outstanding work is collated.

## Tasks

### GHP-001: Create GitHub Project board with required fields

- **Intent:** Set up a GitHub Project (v2) board with the fields defined in the
  trial decision space
- **Expected Outcome:** A project board named "Anvil — Execution Board" with
  columns/fields: Status (Backlog, Ready, In Progress, In Review, Blocked,
  Done), APS Module (text), APS Task (text), Priority (single select:
  High/Medium/Low), Risk (single select: Low/Medium/High), Target Milestone
  (text, optional)
- **Files:** — (GitHub UI / `gh` CLI, no code files)
- **Dependencies:** —
- **Notes:** Requires PR #525 merged before starting.
- **Validation:** `gh project list` shows the project; `gh project field-list`
  shows all required fields
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed

---

### GHP-002: Configure project automation

- **Intent:** Set up built-in GitHub Project workflows for automatic status
  transitions
- **Expected Outcome:** Three automations active: (1) PR opened on linked issue
  → issue moves to "In Review", (2) PR merged on linked issue → issue moves to
  "Done", (3) `blocked` label added → issue moves to "Blocked"
- **Files:** — (GitHub Project settings, no code files)
- **Dependencies:** GHP-001
- **Validation:** Open a test PR linked to a test issue; verify status
  transitions fire
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed

---

### GHP-003: Backfill GH issues for in-progress APS work items

- **Intent:** Create GitHub issues for all currently in-progress APS tasks and
  unlinked open PRs, add them to the project board with correct field values
- **Expected Outcome:** GH issues created and linked for:
  - MAINT-005 (JSON output formatting, PR #517)
  - MAINT-006 (Nx generator for CLI commands, PR #516)
  - MAINT-007 (Nx generator for gate checks, PR #516)
  - MAINT-008 (Spinner/progress patterns, PR #517)
  - STACK-018 (Retroactive evidence capture, PR #518)
  - STACK-019 (Missing deliverable audit, PR #518)
  - Open PR #523 (fix: guard against undefined planPathOrId)
  - Open PR #524 (fix: remove redundant option annotation)
  Each issue body includes `APS Module:` and `APS Task:` fields. Each issue
  added to project board with correct Status, APS Module, APS Task, and Priority.
- **Files:** — (`gh issue create` + `gh project item-add`, no code files)
- **Dependencies:** GHP-001
- **Validation:** `gh issue list --state open` shows all backfilled issues;
  project board shows them in correct columns
- **Confidence:** high
- **Priority:** High
- **Status:** Proposed

---

### GHP-004: First reconciliation checkpoint

- **Intent:** Verify APS↔GH parity for all backfilled items after 3–4 days of
  the trial
- **Expected Outcome:** A reconciliation log documenting: (1) any APS tasks
  missing GH issues, (2) any GH issues missing APS linkage, (3) any status
  mismatches between APS and GH Project board. All drift corrected.
- **Files:** — (manual check, results noted in trial retro)
- **Dependencies:** GHP-003
- **Validation:** Zero unresolved drift items after checkpoint
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed

---

### GHP-005: End-of-trial retrospective

- **Intent:** Measure trial outcomes against success criteria from the decision
  space document
- **Expected Outcome:** A retrospective document covering: (1) lead time (issue
  open → merge), (2) drift incidents (APS/GH mismatch count, target ≤10%),
  (3) admin overhead (low/medium/high), (4) visibility quality in Projects.
  Written to `docs/plans/` or `plans/reviews/`.
- **Files:** `docs/plans/gh-projects-trial-retro.md`
- **Dependencies:** GHP-001, GHP-004
- **Notes:** Time-gate — should not begin until at least 2 weeks have elapsed
  since GHP-001 completion, to allow sufficient data collection.
- **Validation:** Document exists with quantitative data for all 4 metrics
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed

---

### GHP-006: Backfill GH issues for additional outstanding work

- **Intent:** As remaining outstanding work is collated, create GH issues and
  add them to the project board
- **Expected Outcome:** All outstanding work items identified by @aneki have
  corresponding GH issues with APS linkage and correct project board fields.
  This task is intentionally open-ended — scope expands as items are collated.
- **Files:** — (`gh issue create` + `gh project item-add`, no code files)
- **Dependencies:** GHP-001
- **Notes:** Scope expands as items are collated by owner.
- **Validation:** Every collated work item has a linked GH issue on the board
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed

---

### GHP-007: Document reconciliation runbook

- **Intent:** Write a short runbook for the twice-weekly APS↔GH reconciliation
  so it can be followed consistently
- **Expected Outcome:** A runbook covering: what to check (APS In Progress with
  no GH In Progress, GH In Progress with no APS In Progress, merged PRs missing
  APS status updates), how to fix each type of drift, and estimated time (target
  15 min).
- **Files:** `docs/guides/aps-gh-reconciliation.md`
- **Dependencies:** GHP-004
- **Notes:** Informed by first checkpoint experience from GHP-004.
- **Validation:** Runbook is followable end-to-end in ≤15 minutes
- **Confidence:** high
- **Priority:** Low
- **Status:** Proposed

---

### GHP-008: Decision gate — adopt, adjust, or abandon

- **Intent:** Make an explicit decision on whether to continue with the
  dual-primary model based on trial results
- **Expected Outcome:** A decision record documenting: (1) trial outcome
  (pass/fail against success criteria), (2) decision (adopt as-is, adjust
  contracts/automation, or revert to single-primary), (3) next actions if
  adopting (e.g. expand to all modules, add automation).
- **Files:** `plans/decisions/013-gh-projects-trial-outcome.md`
- **Dependencies:** GHP-005
- **Validation:** Decision record exists; next actions are clear
- **Confidence:** high
- **Priority:** Medium
- **Status:** Proposed
