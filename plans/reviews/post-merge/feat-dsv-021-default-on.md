# Post-merge: feat-dsv-021-default-on

PR: #2473
Branch: `feat/dsv-021-default-on`
APS: DSV-021
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile `plans/index.aps.md` — deliberately excluded from this
      PR to avoid colliding with the held #2472. Fold ALL of: GV2-024 #2470 +
      GV2-025 #2459 Merged, DSV-021 Done, and the NBI re-rank (default-on now
      shipped) into one update; then close/supersede #2472. (agent: yes)
- [ ] Step 2 — Reset the **shared main checkout** working tree: it still holds the
      original uncommitted DSV-021 changes (this PR was patch-copied, not moved).
      After merge those are redundant — confirm `git -C <main> diff` against the
      new `origin/main` is empty/no-op, then discard the stale working-tree copy
      (incl. the unrelated `.deepsec/data/anvil-001/tech.json` reformat, which was
      NOT part of this PR). (human/agent — confirm ownership first)
- [ ] Step 3 — Verify on a box with a live daemon: unset `ANVIL_WATCH_DAEMON` +
      `anvil watch --action check` routes through the daemon; `anvil status`
      shows the Save-time line. With no daemon: subprocess path, no warnings.
      (human required — needs a live daemon)

## Notes

- **Delivers the last v0.8.0-beta cut criterion**: default-on save-time daemon
  routing with ADR-075 rollout controls. `DefaultOnWhenLive` routes only when a
  live daemon answers the `workspace_status` probe (daemon-presence guard — no
  warning storm for daemon-absent installs); `0/false/off/no` opts out;
  `1/true/on/yes` forces the preview path.
- **Adversarial review** confirmed the safety claim (silent, startup-time probe;
  no per-save latency; no auto-start). Addressed: pinned + documented the
  empty/unrecognised-value behaviour (treated as unset, not a silent disable) with
  tests. **Open MINOR follow-up**: `build_save_time_client`'s `DefaultOnWhenLive`
  guard branch has no *direct* unit test (indirectly covered by the FakeTransport
  probe-fail test + the `daemon_routing_mode_from` tests) — extract a
  transport-injectable helper when next touched.
- This PR was extracted (patch-copy) from uncommitted work parked in the shared
  main checkout; `index.aps.md` and `.deepsec/.../tech.json` were intentionally
  left out (see Steps 1–2).
- Local verification: 1944 anvil-cli tests, the `daemon_routing_*` unit tests,
  rustfmt + clippy `-D warnings` + oxfmt + `aps:active-lint` all green.
