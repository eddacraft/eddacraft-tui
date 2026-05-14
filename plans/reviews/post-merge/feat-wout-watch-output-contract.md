# Post-merge: feat/wout-watch-output-contract

PR: #1554
Branch: `feat/wout-watch-output-contract`
APS: WOUT
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] After merge, advance WOUT module narrative state in
  `plans/index.aps.md` from `Done` (schema) to `Merged` once #1554 is on
  `main`. (agent: yes)
- [ ] On the next `0.7.x-beta` release tag containing this commit, advance
  WOUT to `Released/Shipped` in the index narrative and link the release
  record. (agent: yes)
- [ ] Confirm the consumer guide at
  `docs/public/anvil/integrations/watch-output.md` renders correctly in
  the next docs site build — fixture lines are wide; check no overflow
  on the published page. (human required)
- [ ] Spot-check that the docs site sidebar picks up the new
  `watch-output` page under integrations (sidebar_position: 4). (human
  required)
- [ ] If a future PR adds a new `EventPayload` variant in
  `anvil-kernel-types`, ensure the wire spec's "Adding a new event
  variant" rule is followed and the new variant introduces a unique
  required field name. (human required — discovery moment, not a
  scheduled task)

## Notes

The forward-compat trap to watch for in the next 90 days: a developer
adding `EventPayload::ActionResult { ... }` (the reserved-but-not-yet-
emitted variant the spec mentions) MUST also bump `WatchEventPayload`,
`WatchEventType`, and the conversion in
`crates/anvil-kernel-types/src/watch_event.rs::from_engine_event`. The
exhaustive match there will fail to compile if they only touch the
kernel side, which is the intended trip-wire.

The integration test in `crates/anvil-cli/tests/watch_json_output.rs`
is Unix-only by design. Watch behaviour on Windows is covered by the
WOUT-002 unit tests (cross-platform serde). If a future contributor
asks why no Windows integration test, the `cfg(not(target_os =
"windows"))` block plus the file-level comment explain it.

Council review evidence: mini tier (general + adversarial). All
CRITICAL/MAJOR findings addressed before push; minors landed in the
same commit. No outstanding council follow-ups carried forward.
