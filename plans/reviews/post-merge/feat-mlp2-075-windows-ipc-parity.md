# Post-merge: feat-mlp2-075-windows-ipc-parity

PR: #1836
Branch: `feat/mlp2-075-windows-ipc-parity`
APS: MLP2-075
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Windows CI green on the two new tests
      `windows_query_protection_claim_returns_some_when_daemon_attests_worktree`
      and `windows_query_protection_claim_returns_none_when_pipe_absent`
      under `crates/anvil-cli/src/mcp/validation.rs` (agent: yes — check
      latest workflow run on `main` after merge)
- [ ] APS MLP2-075 stub on the `daemon-activation` branch advances
      `Proposed → Merged` (agent: yes — stub is at
      `plans/modules/multilayer-protection-v2.aps.md` Group E; advance
      when the corresponding daemon-activation PR also merges)
- [ ] Smoke check: pull `main` on a Windows + Scoop dev box, run the
      `anvil` binary, confirm `anvil mcp` validate_write responses
      include a non-null `protection_claim` field when the intercept
      daemon is running and attests the current worktree (human
      required — needs Windows host with daemon installed)
- [ ] Confirm GH #1831 stays open until MLP2-051f also lands (agent:
      yes — verify by re-reading #1831 status; 075 alone is the
      plumbing layer, 051f is what surfaces `Protecting` in
      `anvil start --verify`)

## Notes

This PR is the hard-gate dependency of MLP2-051f. On its own it doesn't
change any user-visible behaviour — the MCP `validate_write` response
will gain a `protection_claim` field on Windows when a daemon is
running, but most Windows users don't have the intercept daemon running
yet (no auto-start; documented manual invocation is `anvil intercept
start --foreground`).

The user-visible close of #1831 happens when MLP2-051f's activation
diagnostic wire-up ALSO lands. 051f's promotion path uses the same
`query_protection_claim` surface this PR fixes on Windows — without
075, 051f's promotion never fires on Windows.

The drive-by `build_status` arg-list fix in `intercept.rs:1252` is
required for the existing Windows `windows_query_daemon_status_round_trips_against_local_pipe`
test to compile (it had bit-rotted as the function signature gained
`cascade_records`, `cache`, `in_flight_evaluations` args). Without
that fix the test would silently not compile on Windows.

Pre-existing failures on raw `main` HEAD `008114f1` (not introduced
by this PR; verified by stashing changes and rerunning):

- `antipattern::patterns::tests::retired_html_css_patterns_are_absent`
  — PR #1820 reused the `AP-008` ID for TypeScript eval while the
  test's retired-HTML/CSS hardcoded list still contains `AP-008`.
- `commands::check::tests::secret_only_skip_extension_inputs_dont_falsely_mark_scanned`
  — flaky; passes on isolated rerun.

Neither blocks this PR but both should be tracked separately.

## Council reference

Quick session `council-0ed442eb` (converged): 4 findings, 1 CRITICAL
fixed (DaemonStatus name collision in the new Windows test), 2 MINOR
fixed (production path through `with_pipe_name`, doc-comment on the
uncovered failure branch), 1 MINOR deferred (`eprintln` stderr
leakage follows existing Unix precedent — defer to a shim-wide
sanitisation pass).
