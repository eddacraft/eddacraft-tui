# INTD-015 Cross-Session Attribution — Design Pass

**Status:** Accepted (2026-05-21) — design pass that supersedes the
"design pass pending" gate on
[MLP2-071](../modules/multilayer-protection-v2.aps.md). The validation
matrix and the "must / must NOT" lists below are the explicit contract
the implementation slice PR will be reviewed against.
**Date:** 2026-05-21.
**Origin:** Release-council pass 1 defer-with-issue verdict on
[#1722](https://github.com/eddacraft/anvil-001/issues/1722) (operations-reviewer,
2026-05-20).

## Purpose

INTD-015 ships the fan-out filter and contract (`crates/anvil-intercept/src/fanout.rs`)
but no production caller. The follow-up review on #1722 confirmed `Fanout::new`
and `Fanout::with_cross_session_policy` are not invoked from `run_foreground`,
so `enforcement.telemetry.allow_cross_session` is configured-but-ignored — the
same shape class as #1671. The release-council verdict explicitly held the
wire-up back from `v0.7.0-beta` because it touches the IPC accept-loop
(`crates/anvil-intercept/src/ipc.rs`) that INTD-016 and MLP2-025b already
modified in the same cycle, and the regression risk to the headline daemon-
working claim was disproportionate to the gain (default-deny already produces
the documented runtime outcome).

This design pass is the named unblock for that follow-up. It decides the
contract before TDD so the implementation slice can land in a focused PR
without re-litigating the security questions mid-review.

## Scope

In scope:

- which `(rule_id, hash_of_path)` pairs reach which subscribers under what
  operator configuration
- how the cross-session policy interacts with the MLP2-025 spoof cross-check
  (and the MLP2-070 lineage-derivation hardening that this design pass treats
  as a prerequisite, not a co-requisite)
- how the per-startup HMAC salt tracked in
  [`docs/archive/runbooks/v0.6.0-beta-security-note.md`](../../docs/archive/runbooks/v0.6.0-beta-security-note.md)
  §H2 feeds the redaction primitive — what rotates, who holds the key,
  what subscribers observe across a daemon restart
- the `IpcCommand::SubscribeTelemetry` frame shape (new variant in
  `anvil-intercept-proto::IpcCommand`) plus the accept-loop wiring contract
  that mints `SubscriberId` from peer credentials
- the producer wire-up: which call paths must broadcast through
  `Fanout::route` and how per-subscriber output is written without coupling
  the producer to listener lifecycle
- the regression coverage the implementation slice must add

Out of scope (separate work, follow-up tickets where listed):

- back-pressure / slow-consumer handling — owned by INTD-016 budgets;
  this spec assumes the telemetry-lane budget INTD-016 reserves is enough
  for the v1 single-subscriber MCP / driver-client shape and notes the
  remediation hook for the future multi-subscriber case
- secret-detection rule output filtering beyond the existing INTD-015
  `Redact` envelope (ADR-035 redaction-risk follow-on covers that
  pipeline; pinned in
  [`plans/decisions/035-three-pipe-observability-rule.md`](../decisions/035-three-pipe-observability-rule.md))
- the AIGUARD-002 `anvil.diagnostic.v1` envelope — the fan-out remains
  scoped to the `anvil.notification.v1` outer envelope, per the
  "What this module is **not**" section of `fanout.rs:57-70`
- the per-driver `Participating`-mode allowlist (DRVR-007) — INTD-015
  gates visibility, not authority
- the telemetry-side TLS / cross-UID surface — out of scope per AD-4
  (`plans/decisions/015-intercept-loop-enforcement.md`)

## Threat model recap

Same as INTD-015 itself, with two amendments since the original spec landed:

1. **MLP2-025 spoof cross-check is live.** PR #1608 wired
   `IpcListener::with_cross_check_context`. Sessions registered through a
   path the daemon cannot validate against `SO_PEERCRED` lineage end up
   with `degraded:spoofed-attribution` fence reasons. The INTD-015 fan-out
   has historically treated `originating_session_id` as a black-box opaque
   id — the design pass needs to decide whether `degraded:spoofed-
   attribution` sessions should still be allowed to *produce* envelopes
   that reach their own subscriber.
2. **MLP2-070 lineage-anchor daemon-derivation hardening is queued.**
   `register_with_lineage` currently accepts caller-supplied `(pid,
   pid_starttime)`; MLP2-070 will re-derive from `SO_PEERCRED` /
   `/proc/PID/stat`. Until MLP2-070 lands, a same-UID peer can in
   principle seed an attacker-controlled lineage anchor. The fan-out's
   `OwnershipResolver` consults the registry, so a spoofed registry entry
   would let the spoofer "own" telemetry for their planted session id.
   This design pass treats MLP2-070 as a **prerequisite** for enabling
   `telemetry.allow_cross_session = true` in production, not for the
   wire-up itself.

The §H2 unsalted-SHA-256 redaction-hash gap is also live: the v0.7.0-beta
documentation tells operators not to enable `allow_cross_session` precisely
because the redaction hash is rainbow-table-able. The design pass folds
the §H2 fix into the same implementation slice so the operator-visible
flag and the redaction primitive land together — see "HMAC salt" below.

## Decisions

### D1. `IpcCommand::SubscribeTelemetry` frame

New variant on `anvil-intercept-proto::IpcCommand`:

```rust
/// INTD-015 / MLP2-071: telemetry subscription. The daemon mints the
/// SubscriberId from peer credentials on connection; the wire frame
/// carries only the operator-visible filter the client wants to apply
/// on top of the daemon-enforced default-deny filter.
SubscribeTelemetry {
    /// Subset filter the subscriber applies *after* the daemon's
    /// own filter. None = all envelopes the fan-out approves.
    /// `Some(filter)` lets a driver narrow further (e.g. only its
    /// own session ids) without changing the daemon's visibility
    /// boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    filter: Option<TelemetrySubscriberFilter>,
},
/// Symmetric tear-down. Optional — disconnecting the IPC socket
/// also unregisters the subscriber.
UnsubscribeTelemetry,
```

`TelemetrySubscriberFilter` is a narrow struct with `session_ids:
Option<Vec<SessionId>>` and `priority_floor: Option<Priority>` for v1.
The frame must round-trip an empty filter (`None`) and round-trip a
populated filter; the existing serde forward-compat rules
(`#[serde(default, skip_serializing_if)]`) apply. **The frame does NOT
carry `subscriber_id`, `driver_name`, or any identity claim** — those
are minted by the daemon from peer credentials, mirroring the same
defence that `originating_driver_id` already relies on.

Response shape: the IPC connection that sends `SubscribeTelemetry`
flips into subscriber mode and starts receiving `NotificationEnvelope`
NDJSON frames as notifications (`id: None`). Request/response on the
same socket continues to work for control-lane commands; the listener
must multiplex (see D5).

### D2. `SubscriberId` minting

`SubscriberId` is constructed inside the accept loop, never from a
wire-supplied field. Sources:

- Unix: `(uid, pid, pid_starttime, binary_path_hash)` where
  `pid_starttime` comes from `/proc/PID/stat` and `binary_path_hash` is
  the HMAC of the canonicalised `/proc/PID/exe` target under the
  per-startup salt (see D4). `uid` is asserted equal to the daemon's
  own UID per the existing `SO_PEERCRED` gate; rejecting unequal UIDs
  is the existing INTD-002 behaviour, not new here.
- Windows: `(sid, pid, pid_starttime, binary_path_hash)` via
  `GetNamedPipeClientProcessId` + `OpenProcessToken` + `NtQuerySystemInformation`
  start-time lookup. The MLP2-028 follow-up that adds Windows lineage
  support is the lift here; until it lands, the Windows subscribe path
  derives a degraded `SubscriberId` that omits `pid_starttime`. The
  fan-out treats degraded ids the same as full ids for ownership
  resolution — the resolver does not read the tuple internals — but
  the daemon emits a structured warning so operators see that the
  Windows path is on the soft edge of the trust model.

The `SubscriberId` wraps an opaque post-mint string (existing shape,
`fanout.rs:122`); the implementation slice changes only the daemon-side
constructor, not the trait/contract callers see.

### D3. `OwnershipResolver` impl

New production impl in `crates/anvil-intercept/src/fanout.rs` (or a
sibling `fanout_registry.rs` if `fanout.rs` is already too long —
the implementer chooses):

```rust
pub struct RegistryOwnershipResolver {
    registry: Arc<SessionRegistry>,
}

impl OwnershipResolver for RegistryOwnershipResolver {
    fn is_authorised(
        &self,
        subscriber: &SubscriberId,
        originating_session_id: &str,
    ) -> bool {
        // A subscriber owns a session id iff the registry has a
        // session under that id whose subscriber binding matches.
        // The binding is set at session register time (the launcher
        // claims the binding by passing its SubscriberId-shaped tuple
        // alongside RegisterSession) and re-checked on every route.
        self.registry
            .lookup_session(originating_session_id)
            .and_then(|s| s.subscriber_binding.as_ref())
            .is_some_and(|binding| binding.matches(subscriber))
    }
}
```

The `subscriber_binding` field on the registry entry is new. The
launcher / driver client populates it when it registers the session
by sending the `(uid, pid, pid_starttime, binary_path_hash)` tuple
in the same shape the daemon will mint for the subscriber later. The
daemon does NOT trust the wire-supplied tuple — it re-derives the
binding from the connecting peer's credentials at register time and
discards the wire value (same shape as MLP2-070 for the lineage
anchor itself). This closes the binding-spoof variant of #1722.

For v1, a session has exactly one subscriber binding. Future work
(MLP2-071-follow) can extend to multi-subscriber via capability
grants; that surface is out of scope here.

### D4. Per-startup HMAC salt (folds §H2)

`hash_of_path` becomes `hmac_of_path` keyed on a per-startup salt
minted at daemon launch:

- Salt: 32 random bytes from the OS RNG, kept in `Arc<TelemetryRedactionKey>`
  on `DaemonState`. Never persisted to disk, never emitted over IPC,
  never logged. Rotated on every cold start.
- Primitive: `HMAC-SHA256(salt, b"intd015-path-v1\0" || input.as_bytes())`,
  hex-encoded. The label is a fixed domain separator so a future
  reuse of the same salt for a different primitive (e.g. driver-id
  hashing) does not collide.
- Wire shape unchanged: `[redacted:{hex}]` — subscribers do not need
  to update. The `Delivery::Redact` envelope shape stays the same.
- Across-restart correlation: subscribers see different hashes for
  the same path after a cold start. This is **intentional** — the
  §H2 follow-up is explicitly that captured-redaction subscribers
  cannot correlate across daemon lifetimes.

`redact_title`, `redact_grouping`, the worktree/session_id hashing in
`redact_envelope`, and any other callers of `hash_of_path` migrate in
the same change. The existing `fanout.rs:436-441` primitive becomes
the `cfg(test)` shim that test fixtures keep using for deterministic
assertions; production callers go through the keyed primitive on
`Fanout`.

### D5. Producer wire-up + accept-loop multiplex

Producer side — `crates/anvil-intercept/src/lib.rs` `run_foreground`:

1. Construct `Fanout` after `DaemonState` is built:
   ```rust
   let fanout = Arc::new(
       Fanout::with_cross_session_policy(
           Box::new(RegistryOwnershipResolver {
               registry: Arc::clone(&daemon_state.registry),
           }),
           opts.enforcement_config.cross_session_policy(),
       ),
   );
   ```
   Attach to `DaemonState` (new `fanout: Arc<Fanout>` field) so the
   IPC listener and the telemetry producer reach the same instance.

2. The existing `TelemetryEmitter::delivered_envelope_for_decision`
   path stays unchanged. Add a sibling
   `TelemetryBroadcaster::broadcast(envelope)` that takes the envelope
   the emitter built, calls `fanout.route(&envelope)`, and writes each
   `RoutedDelivery` to the matching subscriber connection's outbound
   queue. The broadcaster does NOT call into the emitter — it sits
   beside it so the existing log-sink + in-process consumer paths
   remain decoupled from the IPC delivery surface.

3. Every producer that constructs a notification envelope and was
   previously logging it locally now also calls
   `broadcaster.broadcast`. The wave-1 audit (`fanout.rs:73-99`)
   already documented this contract; the implementation slice owns
   the actual wire-up sites — `delivered_envelope_for_decision` is
   the only one for v1, but the comment in the module doc is moved
   from "until that wiring lands" to "wired here".

Accept-loop side — `crates/anvil-intercept/src/ipc.rs`:

1. After credential gate + manifest handshake, the connection enters
   command-dispatch mode (existing). When a `SubscribeTelemetry`
   frame arrives:
   - Mint `SubscriberId` from the peer credentials cached at accept
     time (`peer_pid` already captured per MLP2-025b is the entry
     point; widen the capture to include `pid_starttime` and the
     binary-path hash).
   - Call `fanout.register(subscriber_id.clone())`.
   - Spawn an outbound writer task wired to a per-connection
     unbounded channel; the broadcaster pushes
     `(SubscriberId, NotificationEnvelope)` pairs to the channel.
   - On the next incoming frame, continue command dispatch. If the
     frame is `UnsubscribeTelemetry` or the connection drops, call
     `fanout.unregister(&subscriber_id)` and join the writer.

2. The listener loop adds one `select!` arm that drains the
   broadcaster's per-connection channel and writes to the socket.
   No new task per envelope; one persistent writer per subscriber
   connection.

3. Failure mode: if the channel fills (slow subscriber), the
   broadcaster drops the envelope for that subscriber with a
   structured warning and increments a `dropped_envelopes` counter
   surfaced via `query_status`. This matches INTD-016's "the daemon
   does not block on a misbehaving peer" rule. Channel cap is
   reused from the INTD-016 telemetry-lane budget.

### D6. Spoof cross-check interaction

The `originating_session_id` on an envelope is set by the daemon from
the change-attribution path. If the producing session is currently in
`degraded:spoofed-attribution` state (MLP2-025), the envelope is still
emitted but with a flag the fan-out can read.

Decision: a degraded-spoofed session's envelopes are **delivered to
its own subscriber** (`Delivery::Allow`) but are **never delivered to
any other subscriber**, regardless of `telemetry.allow_cross_session`.
The rationale is that a spoofed-attribution session might be a session
the daemon classified as not provably-ours; redacting its envelopes
into the cross-session stream gives a same-UID adversary a side channel
to confirm "the daemon thinks this session is spoofed" — which is
itself information they should not be able to extract.

Implementation: the fan-out's `decide` method gains a third input —
the spoof-state of the originating session, read through the resolver.
Add `OwnershipResolver::is_degraded_origin(&self, originating_session_id: &str)
-> bool`. Default impl returns `false` so tests do not have to opt in.
Production `RegistryOwnershipResolver` consults the registry's existing
spoof-state on the session entry (MLP2-025).

The contract addition is small but load-bearing — pin it with a
regression test "spoofed-origin envelope is denied to cross-session
subscriber even with Redact policy" alongside the three existing
INTD-015 cases.

### D7. MLP2-070 dependency

The design pass treats MLP2-070 (lineage-anchor daemon-derivation
hardening) as a **prerequisite for enabling `allow_cross_session = true`
in production**, not for the wire-up itself. The wire-up can land
with `allow_cross_session: false` as the documented operator default
and the existing parsing-but-inert shape becomes parsing-and-effective
on the default. The CHANGELOG "Known gaps" line stays in place,
narrowed: it currently says "the daemon parses `allow_cross_session`
but the fanout subsystem and IPC subscriber surface are not yet wired
into the daemon"; after this slice lands it becomes "the redaction
hash on `allow_cross_session: true` deliveries is per-startup HMAC
keyed; MLP2-070 lineage-anchor daemon-derivation is required before
operators can enable the flag without giving same-UID peers a binding-
spoof path. Track [#1674] + MLP2-070 for that prerequisite."

A separate follow-up ticket — filed as part of this design pass's
implementation slice — should remove the CHANGELOG line entirely
once MLP2-070 merges.

## Implementation slice contract

The implementation slice (one PR, scope-capped) must show:

- `IpcCommand::SubscribeTelemetry` + `IpcCommand::UnsubscribeTelemetry`
  variants in `anvil-intercept-proto::lib.rs` with round-trip tests
  matching the existing INTD-002 forward-compat pattern
- `Fanout` constructed in `run_foreground` and threaded to the
  listener (mirror of the MLP2-025b / MLP2-024 / INTD-016
  wire-ups, with a sibling pin in
  `crates/anvil-intercept/tests/daemon_config_wired.rs`)
- `RegistryOwnershipResolver` in `fanout.rs` or sibling file
- `subscriber_binding` field on the registry entry + daemon-derived
  binding at `RegisterSession` time
- `hmac_of_path` primitive replacing `hash_of_path` in production
  callers; `hash_of_path` retained for `cfg(test)` fixture stability
- Per-connection broadcaster + outbound writer task in `ipc.rs`
- The four INTD-015 cases pinned:
  1. own-session subscribe → `Delivery::Allow`
  2. cross-session subscribe, `allow_cross_session: false` →
     `Delivery::Deny`
  3. cross-session subscribe, `allow_cross_session: true` →
     `Delivery::Redact` with `[redacted:{hmac}]` payload
  4. spoofed-origin envelope → `Delivery::Deny` for cross-session
     subscribers regardless of policy (D6)
- One end-to-end integration test that starts a real daemon, registers
  two sessions, subscribes one driver to telemetry, fires a finding
  through the production path, and asserts the subscriber sees the
  expected `Delivery` outcome for each of the four cases above
- Removal of the `Fanout / cross-session telemetry policy (INTD-015) is
  *not* wired here` paragraph from `lib.rs:285-298`; the doc on
  `with_enforcement_config` advances to "wires the fan-out alongside
  the per-worktree cap and IPC limits"
- CHANGELOG "Known gaps" line narrowed per D7
- `docs/runbooks/v0.7.0-beta-security-note.md` §M1 cross-reference
  updated to point at MLP2-071's resolution

The slice must NOT:

- ship multi-subscriber-per-session — single binding is v1
- ship subscriber filter beyond `session_ids` + `priority_floor`
- modify `anvil.notification.v1` envelope shape
- touch AIGUARD-002 / `anvil.diagnostic.v1`
- attempt to bundle MLP2-070 hardening into the same PR — separate
  ticket, separate review, separate ship signal

## Validation matrix

| Surface | Test artefact | What it pins |
| --- | --- | --- |
| Frame round-trip | `anvil-intercept-proto` unit | `SubscribeTelemetry` + `UnsubscribeTelemetry` serialise / deserialise with the documented forward-compat shape |
| Wire-up pin | `crates/anvil-intercept/tests/daemon_config_wired.rs` (extend) | `run_foreground` constructs a `Fanout` with the resolved cross-session policy and registers a `RegistryOwnershipResolver`; mirrors the MLP2-024 / INTD-016 pins introduced by PR #1721 |
| Default-deny | `fanout.rs` unit (extend) | Spoofed-origin envelope denied to cross-session subscriber regardless of policy |
| HMAC keying | `fanout.rs` unit (new) | Two daemon starts produce different `[redacted:{hmac}]` payloads for the same path input |
| HMAC domain separator | `fanout.rs` unit (new) | The `intd015-path-v1\0` label is in the HMAC input — pin against a fixed-salt + fixed-input test vector |
| End-to-end | `crates/anvil-intercept/tests/` (new file `subscribe_telemetry.rs`) | Real daemon, two sessions, four cases above |
| Slow-subscriber drop | IPC integration | Channel fill triggers `dropped_envelopes` increment without blocking the producer |

## Open questions

1. **Subscriber identity persistence across reconnect.** A driver that
   loses its IPC connection and reconnects re-mints a `SubscriberId`
   that is functionally identical (same peer credentials) but is a
   distinct in-memory object. The `is_authorised` check still passes
   because the binding compares the tuple components, not the wrapper
   identity. Document this in the `RegistryOwnershipResolver`
   doc-comment so a future reader does not "fix" the comparison to
   require identity equality.
2. **Multi-subscriber-per-session.** Editors that expose multiple
   subscriber surfaces (e.g. a VS Code extension + a sidecar MCP
   client) may want both to receive telemetry for the same session.
   v1 binds 1:1; the follow-up shape is a `Vec<SubscriberBinding>`
   plus a capability grant primitive. Not in this slice.
3. **`anvil.diagnostic.v1` integration.** Once AIGUARD-002 unlocks
   the diagnostic envelope, the fan-out's redaction logic will need
   to extend over its content fields. The current `redact_envelope`
   only operates on the notification outer envelope. Defer to the
   AIGUARD-002 follow-up; pin "fan-out does not redact diagnostic
   body" in the unit test so it does not silently start to.

## Sources

- [#1722](https://github.com/eddacraft/anvil-001/issues/1722) — origin
  issue + verdict.
- [`plans/reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md`](../reviews/release-council/2026-05-20-v0.7.0-beta-pre-tag.md)
  — defer-with-issue verdict.
- [`plans/modules/multilayer-protection-v2.aps.md`](../modules/multilayer-protection-v2.aps.md)
  MLP2-071 — Blocked → Ready after this artefact lands.
- [`plans/archive/modules/intercept-daemon.aps.md`](../archive/modules/intercept-daemon.aps.md)
  INTD-015 — original spec.
- [`plans/specs/2026-04-22-notification-telemetry-stream-contract.md`](2026-04-22-notification-telemetry-stream-contract.md)
  — `originating_session_id` / `originating_driver_id` contract.
- [`plans/specs/2026-04-26-diagnostic-envelope-coordination.md`](2026-04-26-diagnostic-envelope-coordination.md)
  lines 222-229 — Subscribers MUST default-deny on unknown session ids.
- [`docs/archive/runbooks/v0.6.0-beta-security-note.md`](../../docs/archive/runbooks/v0.6.0-beta-security-note.md)
  §H2 — per-startup HMAC salt follow-up (folded into this slice).
- [`docs/runbooks/v0.7.0-beta-security-note.md`](../../docs/runbooks/v0.7.0-beta-security-note.md)
  §M1 — lineage-anchor daemon-derivation prerequisite (MLP2-070).
- `crates/anvil-intercept/src/fanout.rs` — existing filter + contract.
- PR [#1721](https://github.com/eddacraft/anvil-001/pull/1721) — the
  wire-up pattern this slice mirrors (per-worktree cap + IPC limits).

## Addendum — Phase 2 producer boundary reconciliation (2026-06-08)

This addendum records the one decision where the Phase 2 implementation
slice deviates from D5 as originally written. It does not re-open any
security decision (D1–D7 stand); it narrows *who ships the producer
emission call sites*.

**What changed since 2026-05-21.** D5 step 2–3 told the slice to wire
the broadcaster at `TelemetryEmitter::delivered_envelope_for_decision`,
calling it "the only [producer] for v1". Verification against `main`
(2026-06-08) shows that emitter has **no production caller** — every
invocation is under `#[cfg(test)]`, and the only live notification
producer (`save_time::emit_assurance_transition`) emits a `tracing`
mirror only and carries no `originating_session_id`
(`save_time.rs:297-336`). So D5's named producer site is not a live
path today.

**Authoritative boundary.** The later
[DSV-044](../modules/daemon-save-time-validation.aps.md) grounding
(2026-06-04) re-drew the ownership line and supersedes D5's producer
clause:

- **MLP2-071 Phase 2 (this slice) owns the broadcaster machinery and
  the subscriber surface** — the `SubscribeTelemetry` /
  `UnsubscribeTelemetry` per-connection JSON-RPC handler →
  `Fanout::register`, the per-connection outbound channel + writer
  task, `SubscriberId` minting, the daemon-derived `subscriber_binding`
  at `RegisterSession`, D6, and a `TelemetryBroadcaster` exposing
  `broadcast(envelope)` (→ `Fanout::route` → per-subscriber delivery,
  drop-and-count on a full channel). Files: `ipc.rs`, `lib.rs`,
  `fanout.rs` (+ a new `broadcaster.rs`).
- **DSV-044 owns the emission call sites** in
  `save_time.rs` / `telemetry.rs` / `fence.rs` that *call*
  `broadcast(...)` from real assurance/fence transitions, including the
  session-correlation threading those producers need.

**Why this is not "an emit with no reader" nor "a reader with no
emit".** This slice ships the reader (subscriber surface) *and* the
broadcaster, and exercises the full delivery path end-to-end: the e2e
test starts a real daemon, subscribes a driver over a real socket, and
fires envelopes through the broadcaster's public `broadcast(...)`
entry — the production delivery machinery — asserting each of the four
D-case outcomes. The broadcaster is live, callable, and tested;
DSV-044 then attaches real transition producers to it. This removes
the circular block the two modules previously held on each other.

**E2E contract adjustment.** The slice's e2e drives `broadcast(...)`
directly (the production delivery path MLP2-071 owns) rather than a
live save-time transition (owned by DSV-044). The
"fires a finding through a live transition" end-to-end lands with
DSV-044, validated by the DSV
"assurance-transition-emits-through-fanout" test the DSV-044 entry
already names.

**Unchanged from the slice contract.** D7's MLP2-070 prerequisite is
now satisfied (MLP2-070 is Released/Shipped via `v0.7.0-beta`), so the
CHANGELOG "Known gaps" line is narrowed per D7 in this slice.

### Phase 2 Council follow-ups (DSV-044 prerequisites)

Surfaced by the full Council pass on the Phase 2 slice. None block this
merge (they have zero runtime exposure until a producer broadcasts —
DSV-044), but they MUST be closed before or alongside DSV-044:

1. **Surface `dropped_envelopes` (+ `subscriber_count`) via
   `query_status`.** The broadcaster counter exists but is not yet on
   `DaemonStatusV1`; it stays `0` until a producer drops, so wiring it
   now would observe nothing. Add it with the producer.
2. **Rate-limit the full-channel drop log.** Currently `debug`-level
   per drop (downgraded from `warn` to avoid a flood); when a producer
   lands, add a transition/threshold `warn` so a stalled subscriber is
   visible without per-event spam.
3. **Binary-path-hash mint component (D2).** v1 mints
   `(uid, pid, pid_starttime)`; add the canonicalised `/proc/<pid>/exe`
   HMAC as defense-in-depth.
4. **Binding identity assumption.** v1's `subscriber_binding` is the
   *registering* peer's minted id, so own-session delivery assumes the
   process that calls `RegisterSession` is the same process that later
   `subscribe`s (D3, 1:1 binding). A topology where the launcher
   registers but a separate editor/MCP process subscribes would mint a
   different id and be denied its own session (fail-closed). Revisit
   with real driver topologies (capability-grant / `Vec<binding>` is the
   open-question #2 shape). Register MUST precede subscribe on the same
   peer.
5. **macOS / Windows mint.** `pid_starttime` is Linux-only, so
   `subscribe-telemetry` returns `-32000` on macOS/Windows today
   (fail-closed, logged server-side); full support is the MLP2-027/-028
   follow-up.

A production-path e2e (real `RegistryOwnershipResolver` + daemon-derived
binding, registrant==subscriber over a socket, asserting own-session
delivery) is best added with DSV-044's producer, since only then can a
real transition drive `broadcast` end-to-end through `run_foreground`.
The binding-match logic itself is unit-pinned
(`registry_ownership_resolver_consults_subscriber_binding`) and the
transport path is socket-tested in this slice.
