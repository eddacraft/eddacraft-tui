# Activation Surface — Daemon Evidence Wire-Up

| Field | Value |
|-------|-------|
| Status | Draft — pending planning council |
| Date | 2026-05-21 |
| Author | @aneki + Claude (Opus 4.7, 1M ctx) |
| Drives | GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831) — `ready_restart_required` stuck after MCP install |
| Coordinates with | MLP2 ([Group D / 051a–051e](../modules/multilayer-protection-v2.aps.md)), LAUNCH-009 / -009.5 (archived), ADR-015, INTD-001..-014 (Complete) |
| Affected crates | `anvil-cli` (`activation/`), `anvil-intercept` (read-only consumer) |
| Risk | Medium — touches the protection-state vocabulary surface |

## TL;DR

The intercept daemon ships and emits `ProtectionClaim` snapshots over IPC. The
MCP `validate_write` and `anvil status` surfaces already consume those claims.
The **activation diagnostic does not**, so `anvil start --verify` is structurally
incapable of returning `protecting` and stays at `ready_restart_required`
forever, even when the daemon is actively enforcing the user's worktree.

Wire `build_protection_claim_from_wire` into `activation/diagnostic.rs`,
promote the relevant MCP client to `McpTier::LiveValidation` when the
daemon reports enforced state for the worktree, and clean up the now-stale
"INTD-only, out-of-scope for v1" comments and copy that predate the daemon
shipping.

## Problem

### What users see

From GH #1831 (two affected Windows + Scoop + PowerShell users):

```
PS C:\Users\matt-\source\repos\afl-line-tipping> anvil start --verify
ACTIVATION
  state: ready_restart_required
  Ready, restart required — restart your editor or agent so the MCP server attaches.
```

User #2 corroborates Claude Code's MCP manager shows the `anvil` MCP
server connected with 8 tools, alongside an unrelated `supabase` MCP also
connected — Claude Code MCP is genuinely working. Restart, `anvil init
--force`, and `scoop uninstall && scoop install anvil` all change nothing.

### What the code does

`crates/anvil-cli/src/activation/diagnostic.rs:211-264` —
`protection_state()` returns `Protecting` if and only if at least one
client is at `McpTier::LiveValidation`:

```rust
if matches!(highest_mcp, Some(McpTier::LiveValidation)) {
    return ProtectionState::Protecting;
}
// …
let restart_pending = matches!(
    highest_mcp,
    Some(McpTier::RestartRequired | McpTier::RestartHandshakeVerified)
);
// … restart_pending → ReadyRestartRequired
```

`grep -rn "LiveValidation" crates/anvil-cli/src/activation/` returns
**zero non-test, non-match-arm callers that set the variant**. The variant
exists, the comparison exists, no production codepath assigns it.

The reason is documented (and now out of date) at
`crates/anvil-cli/src/activation/mcp_client.rs:307`:

> `LiveValidation`: out-of-scope for v1. INTD-only.

This was true when LAUNCH-009 / -009.5 landed (Feb-Mar 2026). It stopped
being true when INTD-001..-014 completed (intercept-daemon archived
**16/16 Complete**; PR #1528 merged 2026-05-14). The daemon now produces
`ProtectionClaim` snapshots over IPC and the wire-up to two of three
consuming surfaces shipped:

| Surface | Reads daemon claim? | Reference |
|---|---|---|
| `validate_write` MCP responses | Yes | `crates/anvil-cli/src/mcp/validation.rs:234` |
| `anvil status` protection-claim section | Yes | `crates/anvil-cli/src/commands/protection_claim_section.rs:42` |
| `activation::diagnostic` (drives `anvil start --verify` + `anvil status --verify` state) | **No** | grep proof above |

This is the same class of bug as **MLP2-025b / PR #1671** — feature
implemented to spec, no caller in the surface that needs it. The MLP2 module
re-spec of 051 on 2026-05-17 split into 051a..-051e for precisely the same
"only `anvil status` renders the typed claim today" reason.

### Why Windows-specific reports

Selection bias. Two early Scoop adopters hit the misleading copy first
because they're testing the fresh-install path without INTD running
in their interactive shell. macOS / Linux Cursor users who follow the
documented start flow without enabling the daemon end up at the same
terminal `ready_restart_required` state but typically don't report it —
they aren't told to expect `protecting`.

The underlying defect is platform-agnostic.

## Goals

1. **`anvil start --verify` reaches `protecting`** when the intercept daemon
   is running and reports enforced state for the current worktree.
2. **Honest fallback** when the daemon is not running or the worktree is
   not enforced: tier stops at `RestartHandshakeVerified`, state stays
   `ready_restart_required`, but the surface copy is updated to name the
   daemon as the missing piece (not "restart your editor" alone).
3. **Diagnostic transparency** — `--verbose` (or equivalent) names what
   Anvil found at each tier so users can self-diagnose the gap.
4. **No new daemon work, no new evidence channel.** Only consume what
   `build_protection_claim_from_wire` already produces.

## Non-goals

- New tiers above `LiveValidation`.
- A new state in `ProtectionState` (e.g. `Attached`). The existing
  ladder already accommodates the daemon evidence — the missing thing
  is the consumer.
- Detecting MCP attachment *without* the daemon (e.g. by having
  `anvil mcp serve` drop a heartbeat). Considered earlier; rejected
  because the daemon now ships and is the canonical evidence source.
- Cross-machine federation or multi-worktree aggregation — already
  covered by the daemon's `DaemonStatus` snapshot shape.

## Proposed change

### Wire path

```
                                                      ┌─ writes ─→ anvil/witness/active.ndjson
                                                      │
   editor (Claude Code / Cursor)                      │           (no change — already wired)
        │ stdio                                       │
        ▼                                             │
   anvil mcp serve  ──────────► validate_write  ──────┘
                                       │
                                       │ IPC query (existing)
                                       ▼
                                 anvil-intercept (daemon)
                                       │
                                       │ DaemonStatus snapshot
                                       ▼
                          build_protection_claim_from_wire ──┬─→ MCP response  (shipped)
                                                             ├─→ anvil status  (shipped)
                                                             └─→ activation::diagnostic  *** NEW ***
                                                                       │
                                                                       ▼
                                                              McpTier::LiveValidation
                                                                       │
                                                                       ▼
                                                         protection_state() = Protecting
```

### Concrete edits

1. **`crates/anvil-cli/src/activation/diagnostic.rs`** — add a function
   `promote_to_live_validation_when_daemon_attests(map, worktree)` that
   runs after `promote_restart_required_after_handshake`:

   ```rust
   fn promote_to_live_validation_when_daemon_attests(
       map: &mut BTreeMap<McpClientId, McpProbeResult>,
       worktree: &Path,
   ) {
       // Cheap query — fails fast when daemon socket isn't reachable.
       let Some(snapshot) = anvil_intercept::status::query_daemon_snapshot() else {
           return;
       };
       let claim = build_protection_claim_from_wire(&snapshot, worktree);
       if !claim_attests_live_enforcement(&claim) {
           return;
       }
       // Promote every client that's already at the handshake tier.
       // The daemon claim attests the worktree, not a specific client —
       // any client that successfully handshaked is the plausible
       // attached client.
       for result in map.values_mut() {
           if result.tier == McpTier::RestartHandshakeVerified {
               result.tier = McpTier::LiveValidation;
           }
       }
   }
   ```

   `claim_attests_live_enforcement` returns true iff the worktree is in
   an enforced state per `WorktreeClaimState` AND the daemon's heartbeat
   in the snapshot is fresh (e.g. last beat within a small bound — TBD
   in council).

2. **`crates/anvil-cli/src/activation/mcp_client.rs:307`** — replace the
   "out-of-scope for v1, INTD-only" comment with a pointer to the new
   diagnostic-side promotion path and the daemon claim contract.

3. **`crates/anvil-cli/src/activation/render.rs`** — update the
   `ReadyRestartRequired` repair hint (line 359-361) to name the daemon
   as the next step when the handshake-verified tier has been reached
   without daemon attestation:

   - Handshake verified + no daemon snapshot → "Start the intercept
     daemon with `anvil intercept start` so pre-write validation can
     attach; otherwise restart your editor and re-run `anvil start
     --verify` to re-probe."
   - Handshake verified + daemon snapshot present + worktree not
     enforced → "Daemon is running but is not enforcing this worktree;
     check `anvil intercept status` for the registered worktree set."
   - No handshake (pre-restart) → current copy is correct.

4. **`crates/anvil-cli/src/commands/start.rs` + `status.rs`** — add a
   `--verbose` / `--why` flag (deferring exact name to council) that
   prints each tier's evidence string when state stalls at
   `ReadyRestartRequired`. Format:

   ```
   ACTIVATION (verbose)
     state: ready_restart_required
     mcp claude_code:
       config:   ~/.claude.json (present, anvil entry matches fresh)
       command:  C:\Users\matt-\scoop\apps\anvil\current\anvil.exe
       handshake: ok (≤ 1s)
       daemon:   not running   ← THIS is the missing piece
     watch: not requested
     baseline: 0 antipattern, 0 secret
   ```

   Acceptance criterion #3 from the issue.

5. **Stale-copy sweep** — `plans/archive/modules/launch-flow-readiness.aps.md`
   lines 843-845, 896, 904, 975, 1063 carry the "INTD-only, future PR"
   line. Add a 2026-05-21 status note pointing to this spec so the
   archived module doesn't mislead future reviewers. (Archived modules
   are read-mostly — appended status notes are the convention here.)

### Failure modes & their states

| Daemon | Worktree enforced? | Highest MCP tier reached | `protection_state` |
|---|---|---|---|
| Running | Yes | `LiveValidation` (promoted) | `protecting` |
| Running | No | `RestartHandshakeVerified` | `ready_restart_required` |
| Not running | — | `RestartHandshakeVerified` | `ready_restart_required` |
| Running, IPC times out | — | `RestartHandshakeVerified` | `ready_restart_required` |
| Daemon snapshot heartbeat stale | — | `RestartHandshakeVerified` | `ready_restart_required` |

The honesty contract holds: `protecting` is claimed only when there's
live daemon attestation for *this* worktree.

## Implementation slice

- **Scope:** ~80-150 production lines in `activation/diagnostic.rs`
  (new promotion fn + one call site) + matching test updates +
  `render.rs` repair-hint branching + `--verbose` printer +
  comment / archived-status note sweep. Estimate ≈ 6 files, 200-350
  lines net, mostly additive.
- **Test surface:**
  - Activation diagnostic unit tests already build
    `McpTier::LiveValidation` synth diagnostics (4 existing fixtures).
    Reuse them as the "daemon attests" branch.
  - New test: daemon snapshot mock returns enforced worktree → promotion
    fires.
  - New test: daemon snapshot mock returns unenforced worktree →
    promotion does not fire.
  - New test: daemon unreachable → promotion does not fire, no error
    surfaces.
  - End-to-end: integration test in
    `crates/anvil-cli/tests/protection_claim_cross_surface.rs` (already
    has the cross-surface harness) — add an activation row.
- **Validation order:**
  1. Unit tests for the promotion function.
  2. Render-hint regression test for each `protection_state` /
     daemon-state combination.
  3. End-to-end `anvil start --verify` against a daemon stub.
  4. Manual smoke on Windows + macOS + Linux (Cursor + Claude Code),
     daemon running and not.

## Open questions for council

1. **Heartbeat freshness window.** What's the right bound? The daemon
   snapshot carries timestamps already; council should pick a value
   that's tolerant of a paused/swapped laptop but tight enough to not
   claim `protecting` against a dead daemon. 60s? 5min? Configurable?
2. **Client attribution.** When the daemon attests the worktree (not a
   specific MCP client), should we promote every handshake-verified
   client, the first one, or require a per-client signal? See
   §"Concrete edits" #1.
3. **`--verbose` flag name.** `--verbose`, `--why`, `--explain`,
   `--diagnose`? There's existing precedent in `anvil doctor` to mirror.
4. **APS placement.** This deserves an APS work item. Most natural home
   is MLP2 Group D (the 051 family that's already about typed-claim
   propagation across surfaces) — possibly as MLP2-051f. Confirm.
5. **`ReadyRestartRequired` vs new `Attached` state.** The earlier
   triage proposed an intermediate `Attached` state. With the daemon
   shipping, that proposal collapses into "just promote to
   `LiveValidation`" — but is there a case for keeping `Attached` as
   "daemon present but not yet observed validate_write here"? Likely no
   in v1, but worth challenging.

## Risk + rollback

- **Risk: false-positive `protecting` claim.** If the daemon claim's
  worktree match is loose (e.g. parent path coincidence), the surface
  would overclaim. Mitigation: rely on the existing
  `build_protection_claim_from_wire(snapshot, worktree)` matching
  logic — same predicate the MCP and status surfaces use. If that
  logic is wrong, both other surfaces are already wrong.
- **Risk: IPC latency on `--verify`.** Daemon query adds an IPC
  round-trip to the verify path. Mitigation: keep the query bounded
  (≤ 200ms timeout), and only run it when at least one client is at
  `RestartHandshakeVerified` (same gate the handshake probe uses).
- **Risk: stale daemon snapshot.** Heartbeat freshness check (see open
  question #1).
- **Rollback:** single feature-flag-style guard on the promotion call
  is one line; revert the promotion call to disable. Or remove the
  function entirely — pure additive, no downstream consumers depend
  on the upgrade path beyond user-visible state copy.

## Out of scope

- MCP-serve-side heartbeat (option (b) from earlier triage).
  Superseded by this design.
- Honest-copy-only fix without daemon wiring (option (a)).
  Subsumed — the copy update is part of this slice but with a real
  graduation path behind it.
- INTD-015 cross-session attribution policy (already speced
  separately, 2026-05-21).
