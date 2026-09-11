# Post-merge: feat-mlp2-051f-activation-daemon-evidence

PR: <!-- filled when opened -->
Branch: `feat/mlp2-051f-activation-daemon-evidence`
APS: MLP2-051f
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] CI green on the new activation tests under
      `crates/anvil-cli/src/activation/daemon_evidence.rs` and the
      MLP2-051f `repair_hint` regression tests under
      `crates/anvil-cli/src/activation/render.rs` (agent: yes —
      check the latest workflow run on `main` after merge).
- [ ] APS module advances MLP2-051f `In Progress → Merged`. The
      release-cleanup agent then advances `Merged → Released/Shipped`
      when `v0.7.0-beta` release evidence catches up. Module total
      stays `62/83` after filing + closure (filing already advanced
      82 → 83; closure does not change the denominator).
- [ ] Smoke check on Windows + Scoop: start the intercept daemon
      (`anvil intercept start --foreground`), run `anvil start
      --verify` in a Scoop-installed project, confirm the diagnostic
      reaches `protecting` (no longer stuck at
      `ready_restart_required`). Closes the user-visible #1831.
      Requires Windows host with the daemon — human required.
- [ ] Smoke check on Linux: same flow as Windows; the wire path is
      shared and the unit tests cover the Unix-socket leg, but a
      manual run pins the production XDG / socket discovery seam
      that the tests deliberately skip (the crate forbids
      `unsafe_code`, so `XDG_RUNTIME_DIR` cannot be overridden in
      tests — see the daemon_evidence module doc).
- [ ] Confirm GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831)
      is closed by the merge (agent: yes — verify the GH issue
      transitions to closed and references this PR; if the PR
      description does not include `Closes #1831`, add a comment
      linking the merge SHA).
- [ ] Verify the activation surface no longer emits the stale
      "INTD-only, out-of-scope for v1" copy anywhere. `grep -rn
      "INTD-only" crates/anvil-cli/src/activation/` should return
      zero hits in the live tree. Archived plan modules under
      `plans/archive/` are intentionally untouched per the council's
      "drop the archived-module sweep" verdict.
- [ ] MLP2-051g (`--why` / `--verbose` flag for verbose-mode tier
      printing) is recorded as deferred in the spec but not yet
      filed as an APS work item. Decide whether to file MLP2-051g
      now (when `anvil intercept` operators next want the verbose
      diagnostic) or roll it into the next #1831-class user report.

## Notes

This PR closes GH #1831 — `ready_restart_required` stuck after MCP
install on Windows + Scoop + PowerShell, observed on `v0.7.0-beta`
by two early adopters. The fix is platform-agnostic; Windows
selection bias surfaced it first because fresh-install Scoop users
hit the path without running the intercept daemon, but the same
defect was reachable on macOS / Linux for any user who skipped
`anvil intercept start --foreground` after restarting their editor.

The hard-gate precursors landed first:

- **MLP2-075** (PR #1836) lifted `query_protection_claim` to Windows
  named pipes. Before this, the MCP shim's Windows path returned
  `None` unconditionally; the activation surface would have stayed
  Unix-only without the parity. Merged 2026-05-22.
- **MLP2-051h** added `DaemonStatusV1::generated_at_unix` as a
  daemon-level wall-clock anchor distinct from per-session
  heartbeats. Merged on `main` at `4ec9c5a4` ahead of MLP2-051f so
  the consumer side could rely on the field without a coupled wire
  change. The activation freshness check uses both signals — per-
  session `last_heartbeat_unix` AND the snapshot anchor when
  non-zero — as defence in depth against a daemon whose clock has
  stopped but whose sessions keep heartbeating.

The implementation honours all eight hard gates from the council
session `plan-f4668683`:

1. Worktree canonicalisation (byte-equality with the daemon-stored
   form via `std::fs::canonicalize`).
2. 45-second freshness window on the freshest signal.
3. `WorktreeClaimState` promotion predicate enumerated explicitly
   in `classify_claim`.
4. `ACTIVATION_DAEMON_QUERY_TIMEOUT = 500 ms` — dedicated
   `query_daemon_status_with_timeout` / `_at_with_timeout`
   variants in `commands::intercept`.
5. End-to-end Unix-socket E2E test against a real `IpcListener`
   (Linux). Windows is covered by MLP2-075's existing pipe-bind
   tests in `mcp/validation.rs::tests`.
6. Structured tracing on every promotion + every skip path
   (`tracing::info!` / `tracing::debug!` with documented
   `reason` vocabulary).
7. Render-hint regression coverage in `render.rs::tests` for each
   `DaemonAttestation` variant.
8. Cardinality-based attribution — ≥1 `Participating` surface
   required, tighter than mass-promotion.

The render hint changes are user-visible: `anvil start --verify`
output for `ready_restart_required` now branches on the daemon
attestation outcome:

- Daemon unreachable → `anvil intercept start --foreground`
- Daemon running but worktree unenforced → `anvil intercept status`
- Daemon snapshot heartbeat stale → `anvil intercept restart`
- Daemon `DegradedProtection` all-Quarantined → `anvil intercept recover`
- Daemon `Warming` → wait + re-run
- No handshake-verified client (pre-restart) → original "restart
  your editor or agent" copy preserved

Cumulative diff stat: ~500 lines added across 10 files; ~80% is
tests + APS / spec updates.

Pre-existing failures on raw `main` HEAD `4ec9c5a4` (not introduced
by this PR; same as MLP2-075's post-merge note):

- `antipattern::patterns::tests::retired_html_css_patterns_are_absent`
  — PR #1820 reused `AP-008` for TypeScript eval while the test's
  retired-HTML/CSS hardcoded list still contains `AP-008`. Tracked
  separately; not gating this PR.

## Council reference

Mini session — single-agent review (council-reviewer) on the
implementation branch. Output: 0 CRITICAL, 0 MAJOR, 3 MINOR, 2 NIT.
All actionable minor findings addressed before push:

- E2E test scope: documented in the module doc that the IPC E2E +
  canonicalisation unit tests cover the chain in pieces; the
  through-the-production-entry-point variant is intentionally not
  added because the crate forbids `unsafe_code` and would require
  `std::env::set_var` to redirect `resolve_socket_path()`.
- Missing render tests for `Warming` and `NoParticipatingSurface`
  added (`ready_restart_required_with_warming_says_wait_and_re_run`,
  `ready_restart_required_with_no_participating_surface_points_at_intercept_status`).
- `tempfile::tempdir().keep()` leak replaced with bound `TempDir`
  in `end_to_end_against_real_unix_socket_promotes_to_live_validation`.
- `classify_claim` `DegradedProtection` two-gate contract documented
  in-line.
- Module doc reference to `tests/activation_daemon_evidence.rs`
  corrected to point at the in-module `#[cfg(test)]` location.
