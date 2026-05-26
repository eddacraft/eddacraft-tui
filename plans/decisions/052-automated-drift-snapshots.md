# ADR-052: Automated Drift Snapshot Capture

## Status

Proposed

## Date

2026-05-27

## Context

`anvil drift snapshot` already captures a point-in-time `DriftSnapshot`
(`created_at`, `metrics.boundary_violations`, `violations[]` with
`from_layer`/`to_layer`, antipatterns, suppressions) to
`.anvil/snapshots/snapshot-*.json`, and `anvil drift report` already compares
snapshots and renders a text trend (shipped by the archived DRIFT module). The
machinery exists.

What does **not** exist is any *automatic* capture: the snapshot series is only
populated when a human runs `anvil drift snapshot`. In practice that means the
series is empty or sparse for almost every repo, so:

- The index success criterion **"new cross-boundary edges per sprint decreases
  by 30% within 8 weeks"** has no data to measure against.
- `anvil drift report --since` and the planned `anvil insights --drift`
  sparkline (INSIGHTS-003) have nothing to render and must fall back to
  "insufficient data".

A drift *trend* is a team/sprint-level signal, so the time-series must be
**shared and comparable across the team**, not a divergent per-developer-machine
artefact. Anvil is also local-only (no telemetry, ADR scope guard) — the series
must live in the repo, not an external service. A decision is needed on *what
captures the series and where it lives* before INSIGHTS-003 or a useful
`anvil drift report --since` can be built on real data.

## Decision

Add a **scheduled CI workflow** that captures the canonical drift series:

- New `.github/workflows/drift-snapshot.yml`: weekly `schedule` cron plus
  `workflow_dispatch`, running against `main`.
- The job obtains the `anvil` binary, runs
  `anvil drift snapshot --name weekly-<ISO-week>`, and **opens an auto-merge PR**
  adding the resulting `.anvil/snapshots/snapshot-*.json` to `main`. It does not
  push directly to `main` — the existing trunk ruleset (PR + required checks) is
  respected, no bypass.
- The same workflow prunes `.anvil/snapshots/` to the most recent **26** weekly
  snapshots so the directory does not grow unbounded.

The committed `.anvil/snapshots/` series is the **canonical, team-shared drift
time-series**. Manual `anvil drift snapshot` remains fully supported and simply
adds to the same series. `anvil drift report` and INSIGHTS-003
(`anvil insights --drift`) consume `.anvil/snapshots/` and bucket by
`created_at`; both report "insufficient data" honestly when fewer than two
weekly snapshots exist.

## Rationale

A weekly, `main`-scoped, committed series is the smallest mechanism that
produces a *comparable team metric* while honouring the local-only and
trunk-protection constraints. Weekly cadence matches the per-week granularity
INSIGHTS-003 renders and keeps churn to one small JSON file per week.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Scheduled CI workflow → auto-merge PR (chosen)** | Shared canonical series; respects trunk ruleset (no bypass); weekly cadence = trend granularity; minimal churn; no per-machine divergence | One automated PR/week; workflow must build/obtain `anvil`; only as live as `main` |
| Intercept-daemon timer (local) | No CI cost; works offline | Per-developer-machine series diverge; only runs while the daemon is up; not a team metric |
| Post-commit hook (throttled) | Event-driven, local | Per-developer; too frequent/noisy; post-commit cannot amend the just-made commit, so snapshots land awkwardly |
| CI bot direct-commit to `main` (ruleset bypass) | No weekly PR noise | Requires a trunk-protection bypass for a bot — more privilege, weaker audit trail |

## Consequences

- **Positive:** the drift success criterion becomes measurable; INSIGHTS-003 and
  `anvil drift report --since` get real data without manual discipline; the
  series stays local-only (committed in-repo, no telemetry); trunk protection is
  untouched.
- **Negative:** a recurring automated PR to triage/merge each week; the workflow
  must build or download `anvil` on a schedule; `.anvil/snapshots/` carries
  committed data files (bounded by retention).
- **Risks:** weekly CI build cost; PR noise; a non-deterministic scan would make
  week-over-week deltas misleading.
- **Mitigations:** 26-snapshot retention prune in the same workflow;
  `workflow_dispatch` for on-demand capture; Anvil scans are already
  deterministic by principle, so consecutive snapshots are comparable; if weekly
  PR noise proves unacceptable, revisit the bot-direct-commit alternative under a
  follow-up ADR.

## References

- Related ADRs: ADR scope guard (`docs/vision/anvil-scope-guard.md`)
- APS modules: a new INSIGHTS item (filed with the implementation once this ADR
  is accepted) tracks this capability; INSIGHTS-003 is the consumer; the archived
  DRIFT module (`plans/archive/modules/drift-reporting.aps.md`) shipped the
  underlying `anvil drift snapshot`/`report` machinery
- Success criterion: "new cross-boundary edges per sprint decreases by 30%
  within 8 weeks" (`plans/index.aps.md`)
