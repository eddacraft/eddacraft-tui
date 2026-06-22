# Post-merge: feat-dpo-producer-coverage

PR: #NNN
Branch: `feat/dpo-producer-coverage`
APS: DPO (DPO-001, DPO-002 → Merged)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Flip DPO-001 + DPO-002 to `Merged YYYY-MM-DD via PR #NNN` and the module
      header + index row accordingly; rerun `pnpm aps:index:check` +
      `pnpm aps:active-lint` (agent: yes)
- [ ] Confirm `cargo test -p eddacraft-anvil-intercept -p eddacraft-anvil` green
      on `main` post-merge (agent: yes)
- [ ] Manual: with a live daemon, save a clean file then one with a finding;
      confirm `usage.ndjson` gains a save-time `gate_evaluated` row (the wire
      `kind` value; Pass then Fail) and that file paths are ABSENT by default (no
      `ANVIL_OBSERVATION_INCLUDE_PATHS`) (human required)
- [ ] Manual: trigger a fence engage; confirm exactly one `constraint_applied`
      row with a normalised `reason` and `worktree: "<redacted>"` (paths off)
      (human required)
- [ ] Manual: set `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1`; confirm no DPO rows
      emitted and a startup warn is logged (human required)

## Notes

Council MINOR producer follow-ups are tracked as **DPO-006** in
`plans/modules/daemon-protection-observability.aps.md`. KDS-005 retirement of
the NDJSON writer is tracked in `plans/modules/kindling-daemon-sink.aps.md`.

Producers are activated to the `usage.ndjson` sidecar, extending the USAGE-004
sink contract; this is the interim store until KDS lands the authoritative
kindling daemon/SQLite backend (KDS-004 re-sources views, KDS-005 retires the
NDJSON writer). DPO-003/-004/-005 (read surface + dashboards) remain Blocked on
KDS. Design + review: ADR-088, planning council `plan-a50aa93d`.
