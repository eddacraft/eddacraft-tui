# Activation Surface — Daemon Evidence Wire-Up

## 1. Identifiers and status

| Field | Value |
| ----- | ----- |
| Spec id | `2026-05-21-activation-daemon-evidence-wireup` |
| Status | Draft — planning council complete (session `plan-f4668683`, 4 COUNTER / 1 CONSENSUS); revisions pending |
| Date | 2026-05-21 |
| Owners | @aneki |
| Work item | MLP2-051f (proposed — see §Council Verdicts → APS placement) |
| Supersedes | — |
| Council tier | full (spec review; impl review tier set when MLP2-051f filed) |
| Drives | GH [#1831](https://github.com/eddacraft/anvil-001/issues/1831) — `ready_restart_required` stuck after MCP install |
| Coordinates with | MLP2 ([Group D / 051a–051e](../modules/multilayer-protection-v2.aps.md)), LAUNCH-009 / -009.5 (archived), ADR-015, INTD-001..-014 (Complete) |
| Affected crates | `anvil-cli` (`activation/`), `anvil-intercept` (read-only consumer) |
| Risk | Medium — touches the protection-state vocabulary surface |

> Template note: this draft adopts §1 from `plans/specs/INTEGRATION-SPEC-TEMPLATE.md`
> so the spec is discoverable and cross-referencable. The remaining template
> sections (§3 Data shapes, §4 Message flow, etc.) will be applied in the
> §"Revisions to apply before implementation starts" rewrite that precedes the
> implementation PR — the current content already covers the substance under
> non-template headings (TL;DR, Problem, Goals, Proposed change, Failure modes,
> Implementation slice, Council Verdicts).

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

---

## Council Verdicts (session `plan-f4668683`, 2026-05-21)

5 personas: architect, pragmatic-lead, adversarial-reviewer,
security-analyst, operations-reviewer. Signals: **4 COUNTER, 1
CONSENSUS** (pragmatic-lead). The COUNTERs are not on the wire-path
shape — that's settled — but on **implementation gaps that, left
unaddressed, will reproduce the exact "spec implemented, no callers in
prod" bug this spec is fixing**.

### Convergent verdicts (apply to spec as-is)

1. **Heartbeat freshness window — per-session, not snapshot-level.**
   All five personas independently flagged: `DaemonStatusV1` has **no**
   daemon-level wall-clock timestamp. `HealthStateV1.uptime_seconds`
   is monotonic-since-start, not a freshness signal. The only usable
   anchor is `SessionRecord.last_heartbeat_unix` per-session
   (`crates/anvil-intercept-proto/src/lib.rs:277`).
   - **Decision:** freshness check operates on `max(last_heartbeat_unix)`
     across the worktree's registered sessions, compared against
     `SystemTime::now()`.
   - **Window:** **45 seconds**. Calibrated against the producer cadence:
     `HEARTBEAT_INTERVAL=10s` (`anvil-run/src/heartbeat.rs:22`) +
     `DEFAULT_HEARTBEAT_TTL=30s` registry eviction (`registry.rs:72`) +
     ~5s slack for clock skew / paused-then-resumed laptops. Tighter
     than 30s is unreachable (registry would evict the session first);
     looser than 120s permits a stale-snapshot exploitation window.
   - **Not operator-configurable upward** — security veto (downgrade
     attack surface). May be tightened by config, never loosened.
   - **Side decision:** propose a wire-add of `generated_at_unix: u64`
     to `DaemonStatusV1` (additive, byte-compat with existing
     `#[serde(default)]` shape) as a second consistency check. Filed
     as a precursor sub-task; see §APS placement.

2. **Worktree path canonicalisation is mandatory (was missing).**
   Three personas (architect, adversarial, security) flagged this as
   **critical**. `build_protection_claim_from_wire(snapshot, worktree)`
   does **byte-equality** comparison against `WorktreeStatus.worktree`
   (`crates/anvil-intercept/src/status.rs:667`). The daemon canonicalises
   at registry register-time (`registry.rs:1149`); the activation
   diagnostic does **not**. Symptoms:
   - Caller passes `.` / relative path / symlink → byte-mismatch → claim
     reads `Unprotected` → promotion silently no-ops → user sees
     `ready_restart_required` with no diagnostic. **This is the exact
     failure mode of #1831 reproduced on a different path** — exactly
     what the fix is supposed to eliminate.
   - **Decision:** activation MUST canonicalise its `worktree` argument
     before the IPC call, using the same `std::fs::canonicalize` +
     warn-on-failure pattern as
     `crates/anvil-cli/src/commands/protection_claim_section.rs::fetch_protection_claim_for_cwd`
     (line 72). The daemon canonicalises at register-time inside the
     intercept crate via `DriverManifest::validate_workspace_roots`
     (`crates/anvil-intercept/src/auth.rs` — see the doc-comment at
     line 15 for the register-time contract); the activation-side
     canonicalisation must produce a path that compares equal to
     whatever the daemon stored. Do not invent a parallel
     canonicalisation routine in `activation/`.
   - **Regression test:** register at canonical path, query via symlink,
     assert `Unprotected` (NOT promoted). Register at canonical A, query
     at canonical B that bind-mounts to the same inode but differs in
     path bytes, assert `Unprotected` (paths must literally match;
     inode-equality is **not** protection-equality).

3. **Windows IPC parity is a BLOCKER, not a follow-up.**
   Three personas flagged independently. `crates/anvil-cli/src/mcp/validation.rs`
   lines 190-202 documents that `query_protection_claim` returns `None`
   on Windows because `query_daemon_status_at(&Path)` is Unix-only for
   named pipes. **Both #1831 reporters are Windows + Scoop users.** A
   Unix-only fix does not close the bug.
   - **Decision:** either (a) lift the Windows `_at(&Path)` form (recent
     `feat: impl io::Read/Write for Win32 OwnerOnlyPipeClient` —
     d7873161 — gives us the primitive), (b) extract Windows-capable
     status query as an explicit precursor sub-task, or (c) downscope
     the spec title and re-open #1831 as Windows-blocked.
   - **Pragmatic position:** (a) or (b). Drop the platform-agnostic
     framing from the spec until parity lands.

4. **Test strategy must include a real end-to-end against a daemon
   socket, not just mocks.** Adversarial flagged this as critical, by
   reference to the canonical example: MLP2-025b (PR
   [#1671](https://github.com/eddacraft/anvil-001/pull/1671)) shipped
   `with_cross_check_context` to spec but with zero production
   callers, so the cross-check stayed inert in prod while every unit
   test passed. The activation surface is currently the same shape of
   bug — `LiveValidation` is defined and tested via synth diagnostics
   that pre-insert the variant, but no production path ever sets it.
   Adding a "daemon snapshot mock" inside the unit suite reproduces
   exactly that pattern: the mock satisfies the assertion regardless
   of whether the real IPC call is wired. The fix needs at least one
   test that goes through the real call site.
   - **Decision:** mandatory integration test in
     `crates/anvil-cli/tests/protection_claim_cross_surface.rs` (or a
     new `activation_daemon_evidence.rs`) that: spawns a real daemon
     (or uses the INTD integration test stub), calls `verify()`
     end-to-end with no mock, asserts `protection_state() == Protecting`.
     If the wire-up is absent, this test fails.

5. **IPC timeout — replace aspirational "≤ 200ms" with a real constant.**
   Three personas flagged. Existing `REQUEST_TIMEOUT=2s` on Unix +
   Windows (`intercept.rs:286, :384`); the spec's 200ms bound exists
   nowhere in code. Verify would inherit the 2s budget — 2-4s added
   latency on every interactive `--verify`.
   - **Decision:** new named constant `ACTIVATION_DAEMON_QUERY_TIMEOUT =
     500ms`, dedicated activation-side query function, mandatory unit
     test asserting a hung-daemon stub does not extend verify beyond
     `timeout + 100ms`.

6. **WorktreeClaimState promotion predicate must be enumerated.**
   Architect + ops flagged. `claim_attests_live_enforcement` in the
   spec is hand-waved as "enforced state per WorktreeClaimState" but
   the vocabulary has four non-`Unprotected` values with different
   honesty implications:
   - `PreWriteDaemon` → **promote**.
   - `DegradedProtection` with ≥1 `Participating` surface → **promote**
     (the protection works for non-quarantined sessions).
   - `DegradedProtection` with all surfaces `Quarantined` → **do NOT
     promote** — every session is fenced; surface a dedicated
     "daemon fenced — recover via `anvil intercept recover`" hint.
   - `Warming` → **do NOT promote** (transient state, daemon is
     leaving/joining, not enforcing pre-write).
   - `Unprotected` → **do NOT promote** (already maps to
     `ready_restart_required`).

7. **Structured tracing on every promotion / skip path** is mandatory.
   Security + ops flagged. Without it, false-positive `protecting`
   claims are undiagnosable in support. Mirror the existing pattern in
   `promote_restart_required_after_handshake` (`diagnostic.rs:507-520`):
   - On success: `tracing::info!(worktree, worktree_claim_state, clients_promoted, "activation: promoted to LiveValidation via daemon attestation")`
   - On skip: `tracing::debug!(reason = "daemon_unreachable"|"worktree_unenforced"|"stale_heartbeat"|"platform_gap", "activation: daemon attestation skipped")`

8. **Drop the archived-module sweep.** Architect + pragmatic-lead
   converged. Editing `plans/archive/modules/launch-flow-readiness.aps.md`
   to backfill 2026-05-21 status notes turns the archive into a journal
   and undermines the "archive is settled" convention. The load-bearing
   stale comment is at `mcp_client.rs:307` (live code); update that
   *with an explicit pointer to the new function name and module*, not
   a vague "see the diagnostic-side promotion path" — adversarial
   flagged that vague pointers re-create the MLP2-025b zero-callers
   pattern.

9. **Cut `--verbose` / `--why` flag from this PR.** Pragmatic-lead
   recommended; ops + security flagged operational/safety concerns
   on the verbose output that warrant their own design slice. File as
   **MLP2-051g** (separate work item, ships after #1831 closes).
   - When it lands, name choice: **`--why`** (architect + ops). Mirrors
     `cargo --explain`. `--verbose` collides with log-verbosity
     convention.
   - Verbose output goes to **stderr**, not stdout (scripted consumers
     of `anvil start --verify` parse stdout — see #1831 reporters who
     are clearly running in a scripted shell).

10. **Failure-mode matrix expansion.** Add rows:
    - Windows, daemon running → promotion does not fire (v1 gap, until
      Windows IPC parity lands).
    - Daemon mid-restart (connection accepted then closed) →
      `RestartHandshakeVerified` → `ready_restart_required` (safe). Use
      `.ok()` on the IPC `Result`, never propagate the error.
    - Daemon running, no sessions registered yet (MCP handshake
      verified, daemon was just started) → `ready_restart_required`
      with hint "Daemon running but has not yet observed your editor's
      MCP child; re-run in a few seconds."
    - `DegradedProtection` all-Quarantined → dedicated render hint
      pointing at `anvil intercept recover`.

### Divergent verdicts (council split — recorded, not yet resolved)

**Client attribution policy.** Two positions:

- **Pragmatic / ops / security:** promote every `RestartHandshakeVerified`
  client when the daemon attests the worktree. Honest enough because the
  daemon attests the worktree, not the client; mass-promotion matches
  the per-worktree predicate already trusted by `validate_write` and
  `anvil status`.
- **Architect (with adversarial support):** gate promotion on `≥1
  Participating` `SurfaceClaim` for the worktree. Without that gate, a
  scenario where Cursor is handshake-verified locally but the daemon's
  only registered session is from Claude Code in another shell will
  falsely promote Cursor.

**Recommended resolution (not yet ratified):** adopt architect's tighter
rule. The cost is one `.surfaces.iter().any(|s| matches!(s.state,
SurfaceClaim::Participating))` check; the benefit is preserving the
distinction `LiveValidation` is documented to enforce ("observed from
this client inside this repo"). Mass-promotion violates the documented
invariant even if it's operationally convenient.

**Counter-counter:** the per-client identity isn't actually resolvable
today (daemon's `agent_tag` is not aligned with `McpClientId` labels —
see ARCH-001 follow-up). So the architect's rule is cardinality-based,
not identity-based: ≥1 Participating surface anywhere in the worktree
is the gate, not "this client has a Participating surface." That keeps
the rule implementable today and stricter than mass-promotion.

**To negotiate before implementation starts.** Marked open question
(a-revised) for the steps file.

### New objections deferred to follow-up issues

- **`generated_at_unix` wire-add** (security, adversarial). Propose as
  a one-field additive change to `DaemonStatusV1` with a parity test.
  Either folded into MLP2-051f's precursor checklist or split as
  MLP2-051h. File before the activation wire-up so the consumer can
  rely on it.
- **`PreWriteDaemon` with empty surfaces** (adversarial ADV-005). The
  local-only fallback path in `derive_local_worktree_state`
  (`protection_claim_section.rs:57`) maps `Protecting` to
  `PreWriteDaemon` with empty `surfaces`. After this fix, that mapping
  is reachable from daemon-attested promotion. Audit every caller and
  ensure the daemon-snapshot path is used when available.
- **Witness-chain audit trail** for activation-side promotion (security
  SEC-007). Trace event suffices for v1; track a follow-up to fold
  activation promotions into the witness chain proper.
- **Rollback plan** must enumerate JSON-schema-consuming fixtures
  (adversarial ADV-008). CI fixtures, tutorial snapshots, status.v1
  conformance tests. Not just "remove the function" — add an explicit
  fixture revert list to the steps file.

### APS placement (verdict converged)

**MLP2-051f under Group D**, with the following hard-gates:

1. Windows IPC parity must land before or with `051f` (else 051f does
   not close #1831).
2. `ACTIVATION_DAEMON_QUERY_TIMEOUT=500ms` named constant + dedicated
   query function — not inheriting the 2s `REQUEST_TIMEOUT`.
3. End-to-end integration test against a real daemon socket
   (no mocks); test must fail if the wire-up regresses.
4. Structured tracing on every promotion / skip path.
5. Worktree canonicalisation contract documented and tested.
6. `WorktreeClaimState` promotion predicate enumerated explicitly.

Counters (MLP2 header is currently `62/81`):

- **On filing:** module total advances `81 → 82` (denominator); done-count stays `62` (numerator unchanged — filing is not closure).
- **On merge:** done-count advances `62 → 63`; total stays `82`.

Filing MLP2-051g and -051h would each bump the total again under the same rule.

`--verbose` / `--why` flag → **MLP2-051g** (sibling, lands after 051f).
`generated_at_unix` wire-add → either precursor inside 051f or **MLP2-051h**
(separate, lands before or with 051f).

---

## Revisions to apply before implementation starts

Translating the verdicts above into spec edits (kept here as a checklist
so the steps file references concrete actions):

- [ ] §"Concrete edits" — rewrite #1 to use per-session
      `last_heartbeat_unix` (max across worktree sessions) against
      `SystemTime::now()`; window 45s, not "TBD".
- [ ] §"Concrete edits" — add path canonicalisation step before the
      IPC call, reusing the `fetch_protection_claim_for_cwd` helper
      (`crates/anvil-cli/src/commands/protection_claim_section.rs:72`)
      pattern; ensure the canonical form matches what
      `DriverManifest::validate_workspace_roots` stored at register
      time on the daemon side
      (`crates/anvil-intercept/src/auth.rs`).
- [ ] §"Concrete edits" — enumerate `WorktreeClaimState` promotion
      predicate explicitly (PreWriteDaemon, DegradedProtection cases).
- [ ] §"Concrete edits" — add structured tracing pattern matching
      `promote_restart_required_after_handshake`.
- [ ] §"Concrete edits" — replace the `LiveValidation` comment
      replacement at `mcp_client.rs:307` with a grep-able pointer to
      `promote_to_live_validation_when_daemon_attests` (full module
      path, not vague reference).
- [ ] §"Concrete edits" — **delete** the archived-module sweep (item
      #5). Keep the live-code comment update only.
- [ ] §"Concrete edits" — **delete** the `--verbose` flag (item #4);
      refile as MLP2-051g.
- [ ] §"Failure modes" — add Windows row, daemon-mid-restart row,
      daemon-no-sessions-yet row, DegradedProtection-all-Quarantined
      row.
- [ ] §"Implementation slice" — add `ACTIVATION_DAEMON_QUERY_TIMEOUT`
      constant + dedicated query function + hung-daemon stub test.
- [ ] §"Implementation slice" — promote integration test from
      validation-checklist item to **mandatory hard-gate** with
      explicit "spawns real daemon, calls verify() end-to-end" wording.
- [ ] §"Open questions for council" — mark (a), (b)-recommend-arch,
      (c), (d) as closed; revise (b) into the open client-attribution
      question to negotiate before implementation; remove the
      `Attached` vs `LiveValidation` question (closed: keep
      `LiveValidation`); reframe (e) into the new-objections-deferred
      list above.
- [ ] §"Risk + rollback" — add explicit fixture revert list (CI
      snapshots, tutorial fixtures, status.v1 conformance fixtures).

These revisions are recorded but **not** applied in this commit — the
implementation PR will pull from the revised steps file. Spec stays
in `Draft` until the revisions land.

