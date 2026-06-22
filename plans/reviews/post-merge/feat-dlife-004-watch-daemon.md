# Post-merge: feat-dlife-004-watch-daemon

PR: #2759
Branch: `feat/dlife-004-watch-daemon`
APS: DLIFE-004
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Manually verify the interactive offer in a real TTY (human required):
      in a terminal with no daemon running, `anvil watch` prompts on stderr
      with `No save-time daemon is running. Start one now for daemon-backed
      validation? [Y/n]`. Answering `y` prints
      `anvil watch: daemon: started the per-user save-time daemon; …`;
      answering `n` prints
      `anvil watch: daemon: not started (declined); … Run `anvil start` to
      start it later.` and the watcher still runs on the scoped fallback.
      This path has no CI coverage (no TTY); the planner + render unit tests
      pin the decision and copy — this confirms the wiring end-to-end.
- [ ] Manually verify reuse copy in a real TTY (human required): with a
      daemon already running (`anvil start`), `anvil watch` prints
      `anvil watch: daemon: reusing the per-user save-time daemon already
      running.` and does NOT prompt.

## Notes

DLIFE-004 wires `anvil watch check` to the ADR-082 tiered daemon posture:
prompt-in-TTY, deterministic-fallback-in-headless. The non-interactive
fallback, `--no-daemon`/`ANVIL_WATCH_DAEMON=0` opt-out copy, and `--json`
stdout purity are all covered by automated tests
(`crates/anvil-cli/tests/watch_daemon_lifecycle.rs` +
`commands::watch::tests::watch_plan_*` / `watch_*_line*`). Only the
interactive prompt path needs a human at a terminal.

DLIFE-005 (docs alignment) landed 2026-06-18 via PR #2765.
