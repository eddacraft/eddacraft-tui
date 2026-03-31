# Council Review: RCLI3 Dashboard Spec Templates

**Branch**: `fix/rcli-tier3-json-spec` **Commit**: `f8ac7c61` feat(json-render):
add 3 dashboard spec templates **Reviewer**: council-reviewer (Opus 4.6)
**Date**: 2026-03-31

## Scope

3 JSON dashboard specs in `packages/json-render/specs/` plus a validation test
suite in `packages/json-render/src/specs.test.ts`.

---

## 1. Validity

### Component types

All elements across the 3 specs use only the 12 components registered in
`catalog-registry.ts`:

| Catalog component | gate-summary | watch-session | architecture-health |
| ----------------- | :----------: | :-----------: | :-----------------: |
| Stack             |      Y       |       Y       |          Y          |
| Grid              |      Y       |       Y       |          Y          |
| Heading           |      Y       |       Y       |          Y          |
| Text              |      -       |       Y       |          Y          |
| Badge             |      -       |       -       |          Y          |
| Separator         |      Y       |       Y       |          Y          |
| Table             |      Y       |       Y       |          Y          |
| Alert             |      Y       |       -       |          -          |
| Card              |      -       |       Y       |          -          |
| Progress          |      -       |       Y       |          -          |
| MetricCard        |      Y       |       Y       |          Y          |
| StatusBadge       |      Y       |       Y       |          -          |

No spec references a component outside the catalog. **PASS**

### Prop schemas

Every prop value was checked against the Zod schema in the shadcn catalog
(`@json-render/shadcn/catalog`) and the custom Anvil definitions in
`catalog-registry.ts`. All prop values are valid enum members or correct types.
Notable validations:

- `MetricCard.trend`: all uses are `"up"`, `"down"`, or `"flat"` (valid enum)
- `MetricCard.format`: all uses are `"number"`, `"percent"`, or `"duration"`
- `StatusBadge.status`: uses `"pass"` and `"info"` (valid enum includes
  `pass|fail|warn|info`)
- `Badge.variant`: uses `"destructive"`, `"secondary"`, `"default"` (all valid)
- `Alert.type`: uses `"warning"` and `"info"` (valid enum)
- `Progress.value`/`max`: numeric types as required

**PASS** — all props match their schemas.

### Structural integrity

- All specs have `root`, `elements`, `title`, `description`, `version: "1.0"`
- All child references resolve to existing element keys (no dangling refs)
- All elements are reachable from root (no orphans)
- All leaf elements have `"children": []`

**PASS**

---

## 2. Data Accuracy

### gate-summary vs `crates/anvil-cli/src/commands/gate.rs`

**Source of truth**: `AVAILABLE_CHECKS` at line 61:

```rust
const AVAILABLE_CHECKS: &[&str] = &[
    "lint", "test", "coverage", "dependency",
    "secret", "architecture", "policy",
];
```

`GateResult` struct (line 72): `overall: bool`, `score: f64`,
`checks: Vec<CheckResult>`, `duration_ms: u64`.

`CheckResult` struct (line 80): `name: String`, `passed: bool`, `score: f64`,
`message: String`.

| Finding                                                                                                                                                            | Severity      | Status     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------- | ---------- |
| Spec had 9 checks with invented names (`secret-detection`, `anti-pattern`, `code-complexity`, `commit-message`, etc.) instead of the 7 real AVAILABLE_CHECKS names | **Important** | **Fixed**  |
| Spec used `"warning"` as a status value in the checks table, but `CheckResult.passed` is a `bool` — no intermediate state exists                                   | Minor         | Documented |
| Score in `gate-status` label updated from 87 to 92 to reflect 7/7 passing (gate.rs computes `passed_count / total * 100`)                                          | Minor         | **Fixed**  |
| Warning about "anti-patterns" replaced with dependency audit warning (anti-pattern is not an AVAILABLE_CHECK)                                                      | **Important** | **Fixed**  |

**After fix**: 7 checks using names `lint`, `test`, `coverage`, `dependency`,
`secret`, `architecture`, `policy` — matching `AVAILABLE_CHECKS` order. Messages
aligned with `run_single_check` output (e.g. `"No hardcoded secrets found"`
matches `run_check_secret`'s success message).

### watch-session vs `crates/anvil-cli/src/commands/watch.rs`

**Source of truth**: Kernel `EventPayload` variants (Progress, Snapshot,
Violation, Error) in `crates/anvil-kernel-types/src/events.rs`. TUI data model
in `crates/anvil-tui/src/surfaces/watch/mod.rs`: `WatchData`, `QueuedChange`,
`WatchStats`, `RunHistory`.

| Finding                                                                                                                                                                                                                                                                                     | Severity | Status     |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | ---------- |
| Metrics (Files Watched, Events Processed, Last Check Result) align with `WatchStats.files_watched`, `total_runs`, and `WatchStatus`                                                                                                                                                         | OK       | -          |
| Progress section (6/9 checks, running message) matches `EventPayload::Progress { current, total }`                                                                                                                                                                                          | OK       | -          |
| Events table shows filesystem events (modified/created/deleted) with a "Triggered Check" column, but `QueuedChange` stores `{file, kind, timestamp}` where `kind` is the violation message, not a filesystem event type. The table represents a higher-level view not yet in the data model | Minor    | Documented |

The watch-session spec is a reasonable aspirational target for `QueuedChange`
enrichment. The TUI event adapter would need to track the originating file
change type to populate this view.

### architecture-health vs `crates/anvil-cli/src/commands/architecture.rs`

**Source of truth**: `ValidationResult` struct (line 74): `valid: bool`,
`template: String`, `layers: usize`, `rules: usize`, `issues: Vec<String>`.

`architecture validate` only validates YAML config structure (checks that layer
dependency references exist). It does **not** scan source files for import
violations, compute severity levels, or produce a From/To violation table.

The boundary violation data (From/To/Rule/Severity) and compliance rule summary
(PASS/WARN/FAIL per rule) come from `architecture watch` which processes kernel
`EventPayload::Violation { policy_id, file, symbol, message }` events.

| Finding                                                                                                                                                                     | Severity      | Status                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Description said "Dashboard for anvil architecture validate results" but showed data the command doesn't produce (per-file violations, severity levels, compliance summary) | **Important** | **Fixed** — description now says "combining architecture validate (config validity) and architecture watch (live boundary monitoring)" |
| Violation table columns include From/To but kernel `EventPayload::Violation` only has `file` (single path) and `policy_id` — no "target" field for import destination       | Minor         | Documented                                                                                                                             |
| Violation severity (error/warning/info) not in kernel event model — would need to be derived from policy metadata                                                           | Minor         | Documented                                                                                                                             |

---

## 3. Completeness

### gate-summary

Covers: overall status, per-check results table, metrics (checks run, warnings,
duration), actionable warnings. This matches what `anvil gate` prints in its
non-JSON output (lines 394-420 of gate.rs).

**Missing but acceptable**: per-check score is shown in the table but not
visually emphasised (no progress bars or colour coding). The gate profile used
(dev/ci/production) is not shown — could be useful context.

### watch-session

Covers: session status, file monitoring metrics, in-progress check, recent event
history. This matches the TUI watch surface data model (`WatchData`).

**Missing but acceptable**: run history panel (the TUI has a `RunHistory` list
showing past gate runs with pass/fail and duration). Adding a history table or
sparkline would better match the native TUI.

### architecture-health

Covers: health score, severity breakdown, violation details, rule compliance
summary. This is the richest spec and provides a comprehensive view.

**Missing but acceptable**: layer dependency graph visualisation (the command
has `architecture show` which lists layers and their dependencies). A simple
list of layers with dependency counts would complement the violation table.

---

## 4. Consistency

| Aspect            |            gate-summary            |           watch-session            |        architecture-health         |               Consistent?                |
| ----------------- | :--------------------------------: | :--------------------------------: | :--------------------------------: | :--------------------------------------: |
| Root element      |              `"page"`              |              `"page"`              |              `"page"`              |                    Y                     |
| Version           |              `"1.0"`               |              `"1.0"`               |              `"1.0"`               |                    Y                     |
| Layout            |        Stack(vertical, lg)         |        Stack(vertical, lg)         |        Stack(vertical, lg)         |                    Y                     |
| Header            | Stack(horizontal) + title + status | Stack(horizontal) + title + status | Stack(horizontal) + title + metric |                    Y                     |
| Metrics           |            Grid(3 cols)            |            Grid(3 cols)            |            Grid(4 cols)            | Y (4 is justified by severity breakdown) |
| Detail sections   |          Heading + Table           |  Heading + Table, Card + Progress  |  Heading + Table, Badge+Text list  |          Y (adapted to domain)           |
| Separators        |             horizontal             |             horizontal             |             horizontal             |                    Y                     |
| Element ID naming |             kebab-case             |             kebab-case             |             kebab-case             |                    Y                     |
| File naming       |         `*.dashboard.json`         |         `*.dashboard.json`         |         `*.dashboard.json`         |                    Y                     |

One minor inconsistency: gate-summary and watch-session use `StatusBadge` for
overall status, while architecture-health uses `MetricCard` with
`format: "percent"`. This is justified — a health score is quantitative, not
categorical.

---

## 5. TUIDASH Readiness

### Unblocking status

These specs provide concrete JSON data for:

- **TUIDASH-001** (spec definition): the 3 files demonstrate the spec format
  that `spec.rs` needs to parse
- **TUIDASH-002** (component registry): all 12 catalog components are exercised
  across the specs
- **TUIDASH-003** (renderer): specs show realistic nested element trees for
  render traversal
- **TUIDASH-004** (layout): Grid + Stack patterns demonstrate grid_layout and
  section rendering
- **TUIDASH-005** (data components): MetricCard, Table, and StatusBadge are
  well-represented
- **TUIDASH-008** (data binding): specs provide static placeholder data that
  binding can later replace with live values

### TUIDASH-007 domain components

The file map indicates TUIDASH-007 will add these domain-specific components:

- `gate_result.rs` — GateResult
- `warning_list.rs` — WarningList
- `drift_indicator.rs` — DriftIndicator
- `plan_card.rs` — PlanCard
- `suppression.rs` — Suppression
- `evidence_entry.rs` — EvidenceEntry

**None of these are in the current catalog.** The specs use generic primitives
(Table for violations, Badge+Text for compliance rules, Alert for warnings)
instead. This is correct — the domain components don't exist yet.

**Action needed when TUIDASH-007 lands**: add GateResult, WarningList, etc. to
`catalog-registry.ts`, then update specs to use them for richer domain
rendering. The current generic composition provides a working fallback.

### TUIDASH-006 (charts)

No chart components (line_chart, bar_chart, sparkline_chart) are used in any
spec. A sparkline showing gate pass rate over time in watch-session would be a
natural addition once TUIDASH-006 lands.

---

## Findings Summary

| #   | Severity      | File                               | Issue                                                                                                                                 | Status     |
| --- | ------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| 1   | **Important** | gate-summary.dashboard.json        | Check names didn't match `AVAILABLE_CHECKS` from gate.rs (9 invented names vs 7 real ones)                                            | **Fixed**  |
| 2   | **Important** | gate-summary.dashboard.json        | Anti-pattern warning referenced a check that doesn't exist in the Rust CLI                                                            | **Fixed**  |
| 3   | **Important** | architecture-health.dashboard.json | Description claimed `architecture validate` results but showed violation data the command doesn't produce                             | **Fixed**  |
| 4   | Minor         | gate-summary.dashboard.json        | `"warning"` status in checks table, but `CheckResult.passed` is a bool (no tri-state)                                                 | Documented |
| 5   | Minor         | watch-session.dashboard.json       | Events table shows filesystem events (modified/created/deleted) but `QueuedChange.kind` stores violation messages, not FS event types | Documented |
| 6   | Minor         | architecture-health.dashboard.json | Violation table From/To columns imply import pairs, but kernel `EventPayload::Violation` has only a single `file` field               | Documented |
| 7   | Minor         | architecture-health.dashboard.json | Violation severity levels (error/warning/info) not in kernel event model — needs policy metadata derivation                           | Documented |
| 8   | Minor         | All specs                          | TUIDASH-007 domain components (GateResult, WarningList, etc.) not yet in catalog — specs use generic primitives as fallback           | Documented |
| 9   | Nit           | gate-summary.dashboard.json        | `metric-duration` value `"42"` with format `"duration"` — units ambiguous (seconds? milliseconds? gate.rs uses `duration_ms: u64`). Value updated from `"4.2"` to `"42"` in a prior fix pass.   | Documented |
| 10  | Nit           | All specs                          | TUIDASH-006 chart components not exercised — sparklines would enhance watch-session                                                   | Documented |

---

## Verdict

**Approved with fixes applied.** The 3 Important findings have been corrected in
this review. The remaining Minor/Nit findings are documented for future work and
do not block merge. The specs are structurally valid, use only catalog
components, and provide sufficient data contracts to unblock TUIDASH-001 through
TUIDASH-008.
