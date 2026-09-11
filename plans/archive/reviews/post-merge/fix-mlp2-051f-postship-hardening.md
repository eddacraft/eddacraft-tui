# Post-merge: fix-mlp2-051f-postship-hardening

PR: <!-- filled when opened -->
Branch: `fix/mlp2-051f-postship-hardening`
APS: residual hardening of MLP2-051f (no new work item; covered by
the existing entry plus MLP2-051g/i/j filed in PR #1847).
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] CI green on the five new / updated tests:
  - `far_future_heartbeat_is_rejected_as_stale` (new)
  - `max_future_clock_skew_boundary` (new)
  - `slow_drip_response_does_not_exceed_wall_clock_budget` (new, Linux)
  - `ready_restart_required_with_all_quarantined_points_at_daemon_restart` (updated)
  - `ready_restart_required_with_stale_heartbeat_points_at_daemon_start` (updated)
- [ ] Smoke check on a daemon-attached Linux host: stop the
  intercept daemon mid-session, run `anvil start --verify`, confirm
  the trace output shows `warn`-level "activation: daemon attestation
  skipped (reason=daemon_unreachable)" at the default
  `ANVIL_LOG` setting. Human required.
- [ ] Smoke check on Windows + Scoop: same flow as Linux; the
  Windows IPC path already had a single-deadline wall-clock fix
  (MLP2-051f Copilot review round 1). This PR's Unix per-frame
  deadline brings the two platforms to parity.
- [ ] No CHANGELOG update for `v0.7.0-beta` — this PR ships
  post-release as a hardening point fix.

## Notes

Originated from the full Council review on the MLP2-051f/g/h +
MLP2-075 work-set (2026-05-22). The review surfaced 4 MAJOR
findings that warranted pre-release fixes:

1. **Render hints pointed at subcommands that don't exist.**
   `StaleHeartbeat` and `AllSurfacesQuarantined` named
   `anvil intercept restart` / `anvil intercept recover` — neither
   is registered. Rewrote both to use existing subcommands
   (`anvil intercept start --foreground` + `anvil intercept unblock
   --worktree $(pwd)`). Operations regression risk: a user following
   the printed hint would have hit `error: unrecognized subcommand`.
2. **Skip-path tracing was emitted at `debug` (invisible at default
   `warn` filter).** A user running `anvil start --verify` and
   asking "why isn't this working?" got zero diagnostic output
   without knowing the `ANVIL_LOG=debug` knob exists. The success
   path emits `info` (visible) — failure paths now match the right
   operational level: `warn` for actionable failures
   (`DaemonUnreachable`, `WorktreeUnenforced`, `StaleHeartbeat`,
   `AllSurfacesQuarantined`); `info` for transient states
   (`Warming`, `NoParticipatingSurface`);
   `NoHandshakeVerifiedClient` stays `debug` (genuine pre-restart).
3. **`within_window` accepted unbounded future timestamps.** A
   daemon with a broken RTC stamping `u64::MAX` could permanently
   pass freshness. Combined with the `generated_at_unix == 0`
   downgrade sentinel, an attacker controlling snapshot output
   could defeat both freshness gates. Added `MAX_FUTURE_CLOCK_SKEW
   = 90 s` (2× `HEARTBEAT_FRESHNESS_WINDOW`) — tolerates NTP resync
   and DST jitter, rejects clock runaway.
4. **Unix `set_read_timeout` is per-syscall, not per-frame.** A
   daemon writing one byte every (timeout − 1ms) kept
   `read_until(b'\n')` alive for ~524 s worst case before bail. The
   Windows path had a single-deadline wall-clock fix from
   MLP2-051f's Copilot round 1; this PR brings Unix to parity. Per-
   iter `set_read_timeout(deadline − now)` against a single
   `Instant`-based deadline.

The remaining post-release MAJORs (Windows E2E test gap, Windows
pipe-owner SID check, MCP claim timeout parity, `DaemonAttestation::
Promoted` cleanup, canonicalisation dedupe, freshness-check `HashSet`
perf) are filed for follow-up via PR #1847 (MLP2-051g/i/j) and
remain open for future hardening passes.

## Council reference

Full session on the merged MLP2-051f/g/h + MLP2-075 work-set, run
2026-05-22 against `origin/main` after MLP2-051f had merged. 5
reviewer personas (architect, kernel-maintainer, adversarial-
reviewer, operations-reviewer, pragmatic-lead). Output: 0 CRITICAL,
11 MAJOR (4 converged), 14 MINOR, 5 NIT. No contradictions between
personas. This PR addresses MAJOR findings 1-4 (the four convergent
pre-release blockers). Findings 5-11 are post-release follow-ups.

Pragmatic-lead defence recorded for the record:
- Test/code ratio for MLP2-051f was **proportionate**, not over-
  engineered, given the MLP2-025b "zero-callers" constraint.
- The two Copilot rounds on MLP2-051f were **genuinely orthogonal**
  (code correctness on #1840, APS prose on #1841).
- Adversarial-reviewer protocol should add **"trace every timeout
  constant to verify wall-clock deadline application"** — the mini-
  Council missed the Windows 2× timeout that Copilot caught. The
  Unix per-frame deadline this PR ships is the same class of bug
  on the other platform; the adversarial pass should now catch the
  next instance preemptively.
