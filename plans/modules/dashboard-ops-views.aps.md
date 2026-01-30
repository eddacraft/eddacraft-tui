# Dashboard Operations Views

| ID | Owner | Status |
|----|-------|--------|
| DASHOPS | @eddacraft | Draft |

## Purpose

Implement the operational and administrative dashboard pages: audit trail, plan
management, configuration viewer, diagnostics, role-based view filtering, and
real-time updates. These pages serve team leads, platform engineers, and
compliance roles — they provide the accountability, lifecycle, and system health
views that complement the developer-focused core pages.

## In Scope

- Audit trail: provenance log, user activity, AI tool tracking, environment info
- Plans page: plan list, plan detail with changes/evidence/approvals/execution
- Configuration viewer: `.anvilrc` settings, check configuration, architecture rules
- Diagnostics page: web equivalent of `anvil doctor`
- Role-based view filtering: conditional content by role (developer/lead/platform/security)
- Real-time update infrastructure: WebSocket or SSE for live dashboard data
- Notification indicators: sidebar badges for suppressions expiring, gate failures

## Out of Scope

- Plan approval workflows — requires write API (future)
- Configuration editing — config lives in `.anvilrc` (code-managed)
- External notification delivery (Slack, email) — separate concern
- User authentication — depends on deployment model decision

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalog, data hooks
- `contracts` — `ProvenanceRecordSchema`, `APSPlanSchema`
- `save-time-trust` — Provenance record format
- `drift-reporting` — Plan/gate provenance data
- `suppressions` — Suppression expiry for notifications

**Exposes:**

- Audit pages at `/audit`, `/audit/users`, `/audit/ai-tools`, `/audit/environments`
- Plans pages at `/plans`, `/plans/:id`
- Configuration page at `/config`
- Diagnostics page at `/diagnostics`
- Notification badge system consumed by sidebar navigation
- WebSocket/SSE client for real-time data consumed by all pages
- Role context provider consumed by all pages for conditional rendering

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Audit log is massive on active projects | medium | Server-side pagination; date range filtering required |
| Role-based views add complexity to every page | low | Additive only — roles show/hide sections, never break functionality |
| Real-time updates cause UI flicker | medium | Debounce updates; smooth transitions; don't rerender unchanged data |
| AI tool detection confidence is low | low | Show confidence level; group "inferred" separately from "detected" |

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] DASH foundation tasks are Ready or In Progress

## Tasks

### DASHOPS-001: Audit log viewer with filtering

- **Intent:** Provide a searchable, filterable chronological record of all Anvil activity
- **Expected Outcome:** Paginated table of all provenance records showing:
  timestamp, event type, result (passed/failed), score, trigger type
  (manual/pre-commit/CI/watch/api), scope, file count, duration, user, detected
  AI tool, git branch and short commit hash. Filters: result, trigger, user, AI
  tool, date range, branch. Sorting on all columns. Pagination for large logs.
- **Scope:** `apps/anvil-ui/src/pages/audit/`
- **Non-scope:** User activity breakdown, AI tool analysis
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** Audit log renders with provenance data; filters narrow results; pagination works
- **Confidence:** high

### DASHOPS-002: User activity breakdown

- **Intent:** Understand per-user Anvil usage patterns
- **Expected Outcome:** Grouped view showing per-user metrics: gate runs count,
  pass/fail rate, most common triggers, most active time periods. Useful for
  understanding adoption — which developers are actively using Anvil and how
  frequently. Privacy-conscious: shows aggregate patterns, not individual
  file-level activity.
- **Scope:** `apps/anvil-ui/src/pages/audit/`
- **Non-scope:** Individual file tracking, blame-style views
- **Dependencies:** DASHOPS-001
- **Validation:** User breakdown shows aggregate metrics; all users with provenance records appear
- **Confidence:** high

### DASHOPS-003: AI tool tracking analysis

- **Intent:** Surface which AI coding tools are being used and their quality impact
- **Expected Outcome:** Dedicated view showing: detected AI tools (Cursor,
  Copilot, Claude Code, etc.) with usage frequency, pass rate by AI tool,
  confidence level of detection (high/medium/low/inferred), and trends over
  time. This is a unique Anvil capability — understanding how different AI tools
  affect code quality.
- **Scope:** `apps/anvil-ui/src/pages/audit/`
- **Non-scope:** AI tool configuration, recommendations
- **Dependencies:** DASHOPS-001
- **Validation:** AI tool breakdown renders; pass rates are calculated per tool; confidence shown
- **Confidence:** medium

### DASHOPS-004: Plan list and detail views

- **Intent:** Browse and inspect APS plans through their full lifecycle
- **Expected Outcome:** Plan list table with: ID, intent text, status
  (draft/validated/approved/applied/rolled-back), source
  (cli/api/automation/manual), author, created date, change count, evidence
  count, tags. Filters: status, source, author, date range, tags. Detail view
  shows: header (ID, hash, intent, schema version, status, provenance),
  proposed changes (expandable with path, description, diff preview),
  validations (schema valid, hash verified, check results), evidence trail (all
  bundles with gate results), approval info, execution history
  (apply/rollback/dry-run results).
- **Scope:** `apps/anvil-ui/src/pages/plans/`
- **Non-scope:** Plan creation, approval, execution — read-only views
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** Plan list renders; clicking opens detail; all sections populate with data
- **Confidence:** high

### DASHOPS-005: Configuration viewer

- **Intent:** Display current Anvil configuration in a readable format
- **Expected Outcome:** Page showing current configuration from `.anvilrc` and
  gate configuration: project name, planning directory, format, schema version,
  enabled/disabled checks with thresholds, architecture definition summary
  (template, layers, boundary count), watch patterns, OPA policy configuration.
  All values are read-only — configuration changes require editing `.anvilrc`
  in source control.
- **Scope:** `apps/anvil-ui/src/pages/config/`
- **Non-scope:** Configuration editing
- **Dependencies:** DASH-006
- **Validation:** Configuration page renders current `.anvilrc` values accurately
- **Confidence:** high

### DASHOPS-006: Diagnostics page

- **Intent:** Provide the web equivalent of `anvil doctor` for system health
- **Expected Outcome:** Page showing diagnostic check results: each check with
  pass/warn/fail status, description, and fix suggestion if applicable.
  Environment info: OS, Node version, Anvil version, pnpm version. API
  connectivity status and data freshness indicators (last sync time per data
  domain). A "Run diagnostics" button to refresh checks.
- **Scope:** `apps/anvil-ui/src/pages/diagnostics/`
- **Non-scope:** Auto-fix capability through the web UI
- **Dependencies:** DASH-005, DASH-006
- **Validation:** Diagnostics page shows check results; environment info is accurate
- **Confidence:** high

### DASHOPS-007: Role-based view filtering

- **Intent:** Show different default content based on user role
- **Expected Outcome:** Role context provider that makes the active role
  available to all pages. Four roles: Developer (personal pass rate, warnings in
  their files, suppression reminders), Team Lead (team-level metrics,
  per-developer pass rates, aggregate trends), Platform Engineer (cross-project
  metrics, CI health, tool adoption), Security/Compliance (audit completeness,
  suppression justification quality, evidence coverage). Role selection via
  settings or URL param. Roles are additive — they show/hide optional sections,
  never remove core functionality.
- **Scope:** `apps/anvil-ui/src/contexts/`, `apps/anvil-ui/src/hooks/`
- **Non-scope:** Authentication, access control
- **Files:**
  - `apps/anvil-ui/src/contexts/RoleContext.tsx`
  - `apps/anvil-ui/src/hooks/useRole.ts`
- **Dependencies:** DASH-002
- **Validation:** Switching roles changes visible sections on Overview and Audit pages
- **Confidence:** medium

### DASHOPS-008: Real-time update infrastructure

- **Intent:** Enable live data updates without manual page refresh
- **Expected Outcome:** WebSocket or Server-Sent Events connection from the
  dashboard to the API server. When a running `anvil watch` process produces
  events (file changes, gate results, status updates) or new provenance records
  are created, the dashboard updates automatically. Visual indicator shows
  connection status (connected/reconnecting/disconnected). Fallback to polling
  when WebSocket is unavailable. Updates are debounced to avoid UI flicker.
- **Scope:** `apps/anvil-ui/src/lib/`, `apps/anvil-api/`
- **Non-scope:** Push notifications (browser notifications, Slack, email)
- **Dependencies:** DASH-005, DASH-006
- **Validation:** Starting `anvil watch` in terminal causes dashboard to update without manual refresh
- **Confidence:** low

## Execution

Steps: [../execution/DASHOPS.steps.md](../execution/DASHOPS.steps.md)
