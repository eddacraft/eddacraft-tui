# Post-merge: feat-actmo-019-register-on-start

PR: #3001
Branch: `feat/actmo-019-register-on-start`
APS: ACTMO-019
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Mark ACTMO-019 `Merged YYYY-MM-DD via PR #3001` in
      `plans/modules/activation-mcp-optional.aps.md` (agent: yes)
- [ ] Manual durability check on a real box: `anvil workspace register <wt>
      --persist`, restart the daemon (`anvil intercept stop` then `anvil
      start`), confirm `<wt>` is listed by `anvil workspace list` and the daemon
      logs "registered worktrees from register_on_start config on startup"
      (human required — needs a live daemon; the agent shell cannot reliably run
      `anvil watch`/daemon-bound flows, see inotify-exhaustion note)
- [ ] Windows named-pipe parity: `register --persist` / `list` /
      `unregister --persist` and the startup auto-registration behave the same
      as Unix; display paths free of `\\?\` (human required — dispatch
      `rust.yml` on the branch or run on a Windows box; PR-green Test is
      ubuntu-only)
- [ ] Downgrade-safety spot check: hand-write a `version: 99` workspace.yaml with
      an unknown future key + a real `allow` entry; confirm the daemon serves the
      allow entry (not fail-closed) and logs the dropped-key warning (agent: yes —
      covered by `confinement::tests::newer_format_version_does_not_fail_closed`,
      re-verify on the merged binary)

## Notes

- ACTMO-019 adds the `register_on_start: [paths]` top-level key to the daemon
  confinement config with a version-gated forward/back-compat parser, and has the
  daemon durably register the configured worktrees on startup atop the ACTMO-014
  persisted set. Confinement admission and registration membership stay distinct
  sets.
- The session-id derivation is shared between the CLI client and the daemon
  (`anvil-intercept::registration_store::activation_session_id`) so a configured
  worktree and an `anvil workspace register` of the same path heartbeat one
  membership instead of duplicating. The drift risk is covered by tests in both
  crates; the manual Windows check confirms the dunce `\\?\`-stripping parity.
- Sibling DSV-046 (headless save-time driver) is the consumer of the registry
  membership-change signal; ACTMO-019 only adds a new producer path (startup
  registration), which already emits the existing `Registered` signal.
