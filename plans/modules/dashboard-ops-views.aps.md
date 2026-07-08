# Dashboard Operations Views

| ID      | Owner      | Status | Progress |
| ------- | ---------- | ------ | -------- |
| DASHOPS | @eddacraft | Ready  | 0/7      |

**Last reviewed:** 2026-07-09

## Purpose

Implement the operational and administrative dashboard pages: audit trail, plan
management, configuration viewer, diagnostics, and role-based view filtering.
These pages serve team leads, platform engineers, and compliance roles — they
provide the accountability, lifecycle, and system health views that complement
the developer-focused core pages.

## In Scope

- Audit trail: provenance log with filtering, user activity, AI tool tracking
- Plans page: expanded plan list/detail with changes/evidence/approval context
  building on DASH-011's read-only Plan Driver proof module
- Configuration viewer: `.anvilrc` settings, check configuration, architecture
  summary
- Diagnostics page: web equivalent of `anvil doctor`
- Role-based view filtering: conditional content by role

## Out of Scope

- Plan approval workflows — requires write API and kernel action authority
  design (future)
- Configuration editing — config lives in `.anvilrc` (code-managed)
- Real-time WebSocket/SSE updates — deferred (not needed for local dev tool)
- External notification delivery (Slack, email) — separate concern
- User authentication — depends on deployment model decision; Better Auth/OIDC
  is deferred by ADR-104

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalogue, data hooks,
  dashboard server, and OpenAPI client seam
- `contracts` — `ProvenanceRecordSchema`, `APSPlanSchema` (see `schema-contracts` module)
- `save-time-trust` — Provenance record format [REVIEW: archived module — provenance now emitted by Rust kernel/CLI; verify schema source]
- `drift-reporting` — Plan/gate provenance data [REVIEW: archived module — drift artefacts now produced by Rust kernel; verify schema source]
- `suppressions` — Suppression expiry for notifications [REVIEW: archived module — suppression parser is now Rust per ADR-029; verify schema source]

**Exposes:**

- Audit pages at `/audit`, `/audit/users`, `/audit/ai-tools`
- Plans pages at `/plans`, `/plans/:id`
- Configuration page at `/config`
- Diagnostics page at `/diagnostics`
- Role context provider consumed by all pages for conditional rendering

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Audit log is massive on active projects | medium | Server-side pagination; date range filtering required |
| Role-based views are mistaken for access control | medium | Wave 3 role filtering is presentation-only until a future auth/RBAC design makes it server-enforced |
| AI tool detection confidence is low | low | Show confidence level; group "inferred" separately from "detected" |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] DASH foundation tasks are Ready

## Wave

**Wave 3** — Can begin once Wave 2 is well underway. May overlap late Wave 2.

## Work Items

### DASHOPS-001: Audit log viewer

- **Intent:** Provide a searchable, filterable chronological record of all Anvil
  activity
- **Expected Outcome:** Paginated DataTable of provenance records: timestamp,
  event type, result, score, trigger, file count, duration, user, AI tool,
  branch, commit. Filters: result, trigger, user, AI tool, date range, branch.
- **Files:**
  - `apps/dashboard/src/routes/audit.index.tsx`
  - `apps/dashboard/src/modules/ops/audit/audit-log-table.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Audit log renders with provenance data; filters narrow results;
  pagination works
- **Confidence:** high

### DASHOPS-002: User activity breakdown

- **Intent:** Understand per-user Anvil usage patterns
- **Expected Outcome:** Per-user aggregates: gate runs, pass/fail rate, triggers,
  active periods. Bar chart of runs per user.
- **Files:**
  - `apps/dashboard/src/routes/audit.users.tsx`
  - `apps/dashboard/src/modules/ops/audit/user-activity.tsx`
- **Dependencies:** DASHOPS-001, DASH-004
- **Validation:** User breakdown shows aggregate metrics; all users with
  provenance records appear
- **Confidence:** high

### DASHOPS-003: AI tool tracking analysis

- **Intent:** Surface which AI coding tools are being used and their quality
  impact
- **Expected Outcome:** Detected tools with frequency, pass rate by tool,
  detection confidence, trends over time.
- **Files:**
  - `apps/dashboard/src/routes/audit.ai-tools.tsx`
  - `apps/dashboard/src/modules/ops/audit/ai-tool-analysis.tsx`
- **Dependencies:** DASHOPS-001, DASH-004
- **Validation:** AI tool breakdown renders; pass rates are calculated per tool;
  confidence shown
- **Confidence:** medium

### DASHOPS-004: Plan list and detail views

- **Intent:** Browse and inspect APS plans through their full lifecycle
- **Expected Outcome:** Plan list DataTable (ID, intent, status, source, author,
  date, changes, evidence). Detail: header, proposed changes, evidence trail,
  approval context. Write-capable approval actions remain deferred.
- **Files:**
  - `apps/dashboard/src/routes/plans.index.tsx`
  - `apps/dashboard/src/routes/plans.$id.tsx`
  - `apps/dashboard/src/modules/plans/plan-table.tsx`
  - `apps/dashboard/src/modules/plans/plan-detail.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Plan list renders; clicking opens detail; all sections populate
  with data
- **Confidence:** high

### DASHOPS-005: Configuration viewer

- **Intent:** Display current Anvil configuration in a readable format
- **Expected Outcome:** Read-only view of `.anvilrc` and gate config: checks,
  thresholds, architecture summary, watch patterns, policy config. CodeBlock for
  raw config.
- **Files:**
  - `apps/dashboard/src/routes/config.index.tsx`
  - `apps/dashboard/src/modules/ops/config/config-viewer.tsx`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** Configuration page renders current `.anvilrc` values accurately
- **Confidence:** high

### DASHOPS-006: Diagnostics page

- **Intent:** Provide the web equivalent of `anvil doctor` for system health
- **Expected Outcome:** Environment info, API connectivity, data freshness per
  `.anvil/` domain, config validation. Refresh button.
- **Files:**
  - `apps/dashboard/src/routes/diagnostics.index.tsx`
  - `crates/anvil-dashboard-server/src/capabilities/diagnostics.rs`
  - `apps/dashboard/src/modules/ops/diagnostics/diagnostics-panel.tsx`
- **Dependencies:** DASH-005, DASH-006
- **Validation:** Diagnostics page shows check results; environment info is
  accurate; refresh works
- **Confidence:** high

### DASHOPS-007: Role-based view filtering

- **Intent:** Show different default content based on user role
- **Expected Outcome:** Role context provider (Developer/Team Lead/Platform/
  Security). Dropdown in sidebar footer or `?role=` URL param. Additive — shows/
  hides optional sections. `useRole()` hook. This is not an authorisation
  boundary until a future auth/RBAC design moves role enforcement server-side.
- **Files:**
  - `apps/dashboard/src/contexts/role-context.tsx`
  - `apps/dashboard/src/hooks/use-role.ts`
- **Dependencies:** DASH-001
- **Validation:** Switching roles changes visible sections on Overview and Audit
  pages
- **Confidence:** medium
