# Dashboard Operations Views

| ID      | Owner      | Status | Progress |
| ------- | ---------- | ------ | -------- |
| DASHOPS | @eddacraft | Ready  | 0/7      |

**Last reviewed:** 2026-07-27 — resolved three `[REVIEW]` markers to their
current Rust owners (`crates/anvil-witness/` for provenance, `DriftSnapshot`
for drift, `parse_suppression` per ADR-029 for suppressions). Corrected the
Exposes list: `/plans` and `/plans/$id` already shipped under DASH-011, so this
module extends that surface rather than creating it. Work items unchanged at
0/7.

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
- `crates/anvil-witness/` — the provenance record format. `WitnessLine` is the
  hash-chained ndjson record (each line carries the previous line's
  `compute_line_hash`); `manifest.rs` indexes the archive files.
  `anvil audit-chain` verifies chain integrity via `verify_chain_dag`.
  (Replaces the archived `save-time-trust` module.)
- `DriftSnapshot` in `crates/anvil-cli/src/commands/drift.rs` — plan/gate drift
  data behind `anvil drift`. (Replaces the archived `drift-reporting` module.)
- `parse_suppression` in `crates/anvil-checks/src/antipattern/scanner.rs` —
  suppression records for expiry notifications, authoritative per
  [ADR-029](../decisions/029-suppression-parser-authority.md). (Replaces the
  archived `suppressions` module.)

**Exposes:**

- Audit pages at `/audit`, `/audit/users`, `/audit/ai-tools`
- Plans pages at `/plans`, `/plans/$id` — **already shipped** by DASH-011 as
  the Wave 1 proof module (PR #3321). The path strings are declared in
  `apps/dashboard/src/router.tsx` (`plansRoute`, `planDetailRoute`); the
  components they render live in `apps/dashboard/src/routes/plans.tsx`. This
  module extends them with operator affordances, it does not create them.
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
