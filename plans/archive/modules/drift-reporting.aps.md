<!-- APS: See https://github.com/eddacraft/anvil-plan-spec for format reference -->
<!-- Executable only if tasks exist and status is Ready. -->

# Drift Reporting

| Scope | Owner | Priority | Status |
| ----- | ----- | -------- | ------ |
| DRIFT | —     | medium   | Ready  |

## Purpose

Provide visibility into how the codebase architecture evolves over time. Show
trends in boundary violations, suppressions, and anti-pattern introductions so
tech leads can spot systemic issues before they become expensive to fix.

**Problem:** Without drift visibility:

- Tech leads don't know if architecture is improving or degrading
- Suppression debt accumulates silently
- New patterns emerge without detection until they're widespread

**Solution:** Point-in-time snapshots and comparison reports that show:

- New boundary violations since last snapshot
- Suppression trends (growing or shrinking)
- Anti-pattern introduction rate
- Areas of the codebase with most drift

## In Scope

- `anvil drift snapshot` — Capture current state to file
- `anvil drift compare <old> <new>` — Compare two snapshots
- `anvil drift report` — Generate human-readable report
- Snapshot storage in `.anvil/snapshots/`
- JSON export for CI/CD integration
- Basic trend visualisation (text-based)

## Out of Scope

- Real-time dashboards (v2)
- Team-level attribution (privacy concerns)
- Integration with external analytics (Prometheus, etc.)
- Historical trend analysis beyond two-snapshot comparison
- Automated alerting

## Interfaces

**Depends on:**

- `architecture-safety` — Baseline and edge data
- `antipattern-library` — Anti-pattern counts
- `suppressions` — Suppression tracking

**Exposes:**

- `anvil drift snapshot [--name <name>]` — Capture current state
- `anvil drift compare <snapshot1> <snapshot2>` — Compare snapshots
- `anvil drift report [--since <snapshot>]` — Generate report
- `anvil drift list` — List available snapshots
- `DriftSnapshot` — Snapshot data structure
- `DriftComparison` — Comparison result structure

**Output Example (drift report):**

```
$ anvil drift report --since 2025-01-15

  Drift Report: 2025-01-15 → 2025-01-31 (16 days)

  ────────────────────────────────────────────────────────

  ARCHITECTURE BOUNDARIES

  New violations:     +3  (was 12, now 15)
  Resolved:           -1
  Net change:         +2  ⚠️

  New edges detected:
  • src/api/handlers.ts → src/core/internal.ts (ARCH-001)
  • src/ui/components/Form.tsx → src/api/client.ts (ARCH-002)
  • src/utils/helpers.ts → src/db/queries.ts (ARCH-003)

  ────────────────────────────────────────────────────────

  ANTI-PATTERNS

  New introductions:  +5
  Resolved:           -2
  Net change:         +3  ⚠️

  By type:
  • AP-003 (any):     +2
  • AP-004 (ignore):  +1
  • AP-006 (catch):   +2

  Hotspots:
  • src/legacy/ — 3 new violations

  ────────────────────────────────────────────────────────

  SUPPRESSIONS

  New suppressions:   +4
  Expired:            -1
  Net change:         +3

  Oldest unexpired: 45 days (src/compat/shim.ts)

  ────────────────────────────────────────────────────────

  SUMMARY

  Overall drift:  INCREASING  ⚠️
  Recommendation: Review new violations in src/api/
```

## Acceptance Criteria

- [ ] `anvil drift snapshot` creates timestamped snapshot file
- [ ] `anvil drift snapshot --name release-1.0` creates named snapshot
- [ ] `anvil drift compare` shows added/removed/unchanged counts
- [ ] `anvil drift report` generates human-readable summary
- [ ] Report distinguishes new violations from existing
- [ ] JSON output available with `--json` flag
- [ ] Snapshots stored in `.anvil/snapshots/` directory
- [ ] Snapshot files are portable (no absolute paths)
- [ ] < 1s snapshot generation for typical project

## Tasks

### DRIFT-001: Snapshot schema and storage

- **Intent:** Define snapshot data structure and file format
- **Expected Outcome:** Snapshot schema with versioning, storage utilities
- **Scope:** `core/src/drift/`
- **Non-scope:** Comparison logic
- **Files:**
  - `core/src/drift/snapshot-schema.ts`
  - `core/src/drift/snapshot-storage.ts`
  - `core/src/drift/snapshot-schema.test.ts`
- **Dependencies:** —
- **Validation:** `nx test core --testNamePattern="DriftSnapshot"`
- **Confidence:** high

### DRIFT-002: Snapshot capture

- **Intent:** Capture current state from baseline, warnings, suppressions
- **Expected Outcome:** Service that aggregates current state into snapshot
- **Scope:** `core/src/drift/`
- **Non-scope:** CLI command
- **Files:**
  - `core/src/drift/snapshot-capture.ts`
  - `core/src/drift/snapshot-capture.test.ts`
- **Dependencies:** DRIFT-001, architecture-safety, antipattern-library
- **Validation:** `nx test core --testNamePattern="SnapshotCapture"`
- **Confidence:** high

### DRIFT-003: Snapshot comparison

- **Intent:** Compare two snapshots and identify differences
- **Expected Outcome:** Comparison result with added/removed/unchanged items
- **Scope:** `core/src/drift/`
- **Non-scope:** Report formatting
- **Files:**
  - `core/src/drift/snapshot-compare.ts`
  - `core/src/drift/snapshot-compare.test.ts`
- **Dependencies:** DRIFT-001
- **Validation:** `nx test core --testNamePattern="SnapshotCompare"`
- **Confidence:** high

### DRIFT-004: Report generator

- **Intent:** Generate human-readable drift reports
- **Expected Outcome:** Text and JSON report formatters
- **Scope:** `core/src/drift/`
- **Non-scope:** CLI rendering
- **Files:**
  - `core/src/drift/report-generator.ts`
  - `core/src/drift/report-generator.test.ts`
- **Dependencies:** DRIFT-003
- **Validation:** `nx test core --testNamePattern="ReportGenerator"`
- **Confidence:** high

### DRIFT-005: CLI drift commands

- **Intent:** Add drift subcommands to CLI
- **Expected Outcome:** Working `anvil drift snapshot|compare|report|list`
- **Scope:** `cli/src/commands/`
- **Non-scope:** TUI visualisation
- **Files:**
  - `cli/src/commands/drift.ts`
  - `cli/src/commands/drift.test.ts`
- **Dependencies:** DRIFT-001, DRIFT-002, DRIFT-003, DRIFT-004
- **Validation:** `anvil drift --help && anvil drift snapshot`
- **Confidence:** high

## Decisions

**D-DRIFT-001:** File-based snapshots, not database

- **Rationale:** Simple, portable, version-controllable. No external deps.
- **Alternatives:** SQLite database
- **Trade-offs:** No query capability, but simpler implementation

**D-DRIFT-002:** Two-snapshot comparison only

- **Rationale:** Keeps scope manageable. Multi-point trend analysis is v2.
- **Alternatives:** Time-series database with arbitrary queries
- **Trade-offs:** Limited analysis, but ships faster

**D-DRIFT-003:** No team attribution

- **Rationale:** Privacy concerns, git blame is external tool
- **Alternatives:** Track author per violation
- **Trade-offs:** Less accountability, but avoids blame culture

## Notes

**Snapshot file format:**

```json
{
  "schema_version": "1.0.0",
  "created_at": "2025-01-31T10:00:00Z",
  "name": "release-1.0",
  "metrics": {
    "boundary_violations": 15,
    "antipattern_count": 42,
    "suppression_count": 8
  },
  "violations": [...],
  "suppressions": [...],
  "baseline_hash": "abc123..."
}
```

**CI/CD integration:**

```yaml
# Compare current state against release
- run: anvil drift snapshot --name current
- run: anvil drift compare release-1.0 current --json > drift.json
- run: |
    if [ $(jq '.net_change.violations' drift.json) -gt 5 ]; then
      echo "Too much drift since release"
      exit 1
    fi
```

**Future enhancements:**

- Trend graphs (ASCII or image export)
- Slack/email notifications on drift threshold
- Integration with external dashboards
- Team-level metrics (with opt-in)
