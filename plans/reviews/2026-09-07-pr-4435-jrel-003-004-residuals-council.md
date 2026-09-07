# Council Review — PR #4435 JREL-003/004 residuals

**Status:** Converged
**Tier:** full
**Target:** `main...HEAD` on `cursor/jrel-003-004-residuals-b9e2` (protected:
`crates/anvil-intercept/src/registry.rs`, `crates/anvil-intercept/src/ipc.rs`)
**Date:** 2026-09-07
**PR:** https://github.com/eddacraft/anvil-001/pull/4435
**Contract:** GitHub issue #4433 (Council must-fix residuals from #4428).
#4432 (rendezvous coordinator / physical candidate dedup) is `out_of_scope`.
**Code head reviewed:** `efe7b471ed08dc513f460b7f6dd8836568216120`. Later
commits on this branch are documentation and this review only; protected
surfaces are unchanged after that SHA.

## Change under review

| File | Change | Production impact |
| --- | --- | --- |
| `registry.rs` | Duplicate durable `session.register` fires `MembershipChange::Refreshed`; heartbeat is TTL-only | Same-user socket peer can no longer fork a driver via heartbeat |
| `ipc.rs` | `query_status` snapshot built on the blocking pool | Status no longer parks the single-threaded runtime behind driver-map waits |
| `save_time_driver.rs` | Placeholder stop, lock-free spawn, shutdown wait, leftover adopt, Windows verified kill, boolean lifetime proof | Bounded stop no longer stalls the daemon; restart does not leave two watchers |
| `ensure.rs` | Conflict tally is Answered only; endpoints named | Silent listeners do not wedge same-scope commands |
| `registration.rs` | Readiness wait retries Unknown/Failed; Absent is debug | Daily commands do not warn on a documented opt-out |
| `daemon_recycle.rs` | Same PID with one missing start time is one instance | Legitimate recycle is not refused |
| `watch_driver.rs` | Atomic readiness marker write | Marker write matches its documented contract |
| `anvil-intercept-win32` | `terminate_process_matching` | Recycled Windows PIDs are not terminated |
| Component and cross-system docs | Membership invariant, runbook, diagram-owner freshness | Diagrams unchanged; owners record that disposition |

## Seats

| Role | Verdict | Summary |
| --- | --- | --- |
| general | approve | All five contract clusters are implemented and tested. Remaining gaps are missing coverage on the `WorktreeAlreadyOwned` refresh path, timing-sensitive lock-release tests, and a leftover-path log placeholder. |
| adversarial | approve | Heartbeat spawn is closed; register-path refresh fires under the registry lock; stop/spawn drop the map lock; Windows terminate matches creation time; silent listeners are not live. |
| security | approve | `Refreshed` is reachable only through peer-exe-verified durable `session.register`. Public and IPC heartbeat update TTL only. Windows kill without a start time is a no-op. |
| operations | approve (GO) | Absent is debug; conflict names and logs endpoints; readiness wait retries; marker is atomic; recycle coalesces; stop and status no longer stall the runtime. |
| pragmatic | approve | Scope matches #4433. #4432 is absent. Deferred idle-vs-crash re-stamp and documentary enqueue-only hook are named. Prefer this PR over #4434 if the wire contract must stay `SessionAlreadyExists`. |
| **judge** | **Ship** | No in-contract critical or major findings. |

## Findings

No critical or major findings. In-contract minors and nits are advisory; they
are not `must_fix` for this review.

### In-contract minor (advisory)

- general: the `WorktreeAlreadyOwned` branch of `refresh_duplicate_durable`
  (different session id, same canonical + durable tag) has no dedicated test.
  `registry.rs` ~822.
- general / adversarial: lock-release tests sleep 20 ms then assert elapsed
  time. A `Barrier` or channel would make the race deterministic.
  `save_time_driver.rs` ~2500.
- general: `reconcile_on_start` waits against `Path::new("<leftover>")`, so the
  hard-kill warning cannot name the real PID-file path. `save_time_driver.rs`
  ~544.
- adversarial: permanently silent canonical with no siblings is not covered by
  a dedicated `EnsureOutcome::Failed` test. `ensure.rs` ~523.
- adversarial: Windows `kill` with a missing start time is a silent `Ok(())`.
  A `debug_assert!` would pin the invariant that tracked Windows children always
  have a start time. `save_time_driver.rs` ~285.

### Later / out of scope (not must-fix)

- #4432 rendezvous coordinator and physical candidate dedup.
- Bound cannot distinguish a long-idle child from an immediate crash (no
  re-stamp of readiness while idle).
- Enqueue-only hook contract remains documentary.
- Distinct `DriverStatus::Stopping` wire variant (later_item).
- Distinct error when Windows hard-kill is skipped for a missing start time
  (later_item).
- JSON-RPC `JoinError` text in the status error body (out_of_scope; log
  server-side, fixed string on the wire).
- Competing PR #4434 (`{"ok":true,"refreshed":true}` / additive
  `save_time_driver` JSON). Either approach can satisfy the spawn-authority
  contract; this PR keeps `SessionAlreadyExists` and ADR-094 CLI heartbeat copy.

## Evidence

- Full five-seat Council against issue #4433 and `origin/main...HEAD`.
- Anvil developer MCP tools were unavailable; reviewers used ordinary file
  reads and `git diff`.
- Local (author): `cargo test -p eddacraft-anvil-intercept --no-fail-fast --lib`
  — 1172 passed; targeted CLI unit tests — 9 passed; recovery integration test
  after the spawn/status offload — passed; clippy `-D warnings` on intercept
  and CLI — clean.
- CI on `efe7b471ed08dc513f460b7f6dd8836568216120`: Test, Clippy,
  Clippy (windows-msvc), Check, Format, and editor-coexistence passed. Docs
  corpus failed on diagram-impact (owning-diagram freshness), which this
  follow-up commit settles without touching protected files.

## Decision

**Ship** the JREL-003/004 residuals on PR #4435. A maintainer applies
**`council:reviewed`** on the live head that contains this review and the
protected-path diff at `efe7b471`. Re-council if `registry.rs` or `ipc.rs`
move after the label.
