# Post-merge: feat-insights-003-drift-trend

PR: #2111
Branch: `feat/insights-003-drift-trend`
APS: INSIGHTS-003
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — `anvil insights --drift` exits 0 with no `.anvil/snapshots/`
      present and prints the insufficient-data message (not a panic, not a
      misleading line). (agent: yes — run the built binary in a temp dir)
- [ ] Step 2 — `anvil insights --drift --json` emits a document with
      `schema_version == "anvil.drift_trend.v1"` and a `weeks` array of 8
      entries each carrying `has_data` and `new_edges`. (agent: yes)
- [ ] Step 3 — On a repo with ≥2 calendar weeks of real `anvil drift
      snapshot` history, confirm `anvil insights --drift` renders a
      multi-week sparkline whose per-week counts match the added boundary
      violations between consecutive snapshots, and that weeks with no
      snapshot show as no-data rather than zero. (human required — needs
      genuinely time-spread snapshots that unit fixtures cannot stand in for)
- [ ] Step 4 — Advance INSIGHTS-003 to `Merged ... via PR #2111` and bump
      the Usage Insights module header count 2/4 → 3/4 in
      `plans/index.aps.md` and the module file; drop INSIGHTS-003 from the
      index NBI list (it is no longer "next"). (agent: yes)

## Notes

This feature is intentionally bounded to **reading** the existing drift
snapshot store; it adds no writer and no new on-disk format. The unit tests
(`drift_trend_matches_fixture`, the intra-week-summing and duplicate-id
cases) exercise the bucketing/diff logic with synthetic snapshots, so the
only thing that genuinely needs human post-merge eyes is Step 3 — the
real-world sparkline over snapshots accumulated across actual calendar
weeks, which no fixture can reproduce.

The metric is edge **introductions** per week (deduped by violation `id`),
not a net end-of-week delta: an edge introduced and resolved within the same
week still counts once. If post-merge dogfooding shows users find that
confusing, revisit the metric definition (a net-per-week variant was
considered and deferred — see the INSIGHTS-003 spec reconciliation).
