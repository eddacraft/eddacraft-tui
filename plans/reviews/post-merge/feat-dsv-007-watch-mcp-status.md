# Post-merge: feat-dsv-007-watch-mcp-status

PR: #NNN
Branch: `feat/dsv-007-watch-mcp-status`
APS: DSV-007
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Confirm default behaviour is unchanged: with `ANVIL_WATCH_DAEMON` unset,
      `anvil watch` and `anvil status` produce byte-for-byte the same output as
      before (no daemon probe, no `Save-time:` line, no `save_time` JSON field).
      (agent: yes — diff `anvil status` / `anvil status --json` against pre-merge)
- [ ] Confirm the opt-in path against a live daemon: start the intercept daemon,
      set `ANVIL_WATCH_DAEMON=1`, edit a file under watch, and verify the verdict
      comes from the daemon (no per-save `anvil check` subprocess spawn) and that
      `anvil status` renders the `Save-time:` assurance line. (human required —
      needs a running daemon + interactive watch session)
- [ ] Confirm the fallback: with `ANVIL_WATCH_DAEMON=1` and **no** daemon, a save
      warns once per session and runs a scoped (never `--all`) check; a mid-session
      daemon kill falls back to `unavailable{daemon-absent}`, not a stale `clean`.
      (human required — needs a watch session + daemon kill)
- [ ] DSV-009 / Task 15 follow-up reminder: the cross-path parity gate's
      **"MCP+daemon" leg exercises `scan_buffer`, not `validate_paths`** (MCP is a
      pre-write proposed-content gate). Carry this into the parity corpus design.
      (agent: no — design note for the DSV-009 owner)
- [ ] Advance DSV-007 `In Progress → Merged` in
      `plans/modules/daemon-save-time-validation.aps.md` once this PR merges, and
      `Released/Shipped`/`Complete` per the normal release evidence. (agent: yes)

## Notes

DSV-007 makes `watch` + `status` thin clients of the save-time daemon, all gated
opt-in behind `ANVIL_WATCH_DAEMON` (default-off) because the daemon is not
auto-started in Sub-phase A — so trunk default behaviour must not change. MCP
`anvil_validate_write` keeps the `scan_buffer` verb (council-confirmed
reconciliation of the plan's "validate_paths" wording — see the module's DSV-007
Progress note).

Known, intentionally-deferred items (raised in batch council, not regressions):

- **TUI fallback signal:** in TUI mode the daemon-absent WARN is suppressed
  (the alt-screen owns the screen); the action footer still shows pass/fail.
  Surfacing the daemon-absent reason in the TUI footer is a follow-up.
- **macOS socket round-trip test** is Linux-gated (matching the MCP client's IPC
  fixture tests); the transport itself compiles on all unix targets.
- **`anvil status` IPC budget:** when `ANVIL_WATCH_DAEMON=1` and the daemon
  socket exists but is hung, `status` can add up to the 2s `workspace_status`
  timeout on top of the existing `query_status` budget.
