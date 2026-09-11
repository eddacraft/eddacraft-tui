# Post-merge: fix-cib-153-bind-session-lifecycle-to-peer

PR: [#3188](https://github.com/eddacraft/anvil-001/pull/3188)
Branch: `fix/cib-153-bind-session-lifecycle-to-peer`
APS: CIB-153 (module `continuous-improvement-backlog`)
Merged: <!-- filled by cleanup agent -->
Verified: <!-- filled by cleanup agent -->

## Steps

- [ ] Step 1 — Reconcile CIB-153 status to `Merged YYYY-MM-DD via PR #NNN` in
      `plans/modules/continuous-improvement-backlog.aps.md` (agent: yes).
- [ ] Step 2 — Confirm the ownership tests run in CI's Rust test job (agent:
      yes): `cargo test -p eddacraft-anvil-intercept` shows
      `dispatch_command_heartbeat_binds_to_registering_peer` and
      `dispatch_command_unregister_binds_to_registering_peer` passing, plus the
      registry-level `peer_ownership_check` unit tests (mismatch / no-credential /
      no-lineage / survives-narrowing / owner-only removal / unknown-id
      idempotent).
- [ ] Step 3 — Decide whether to file a follow-up CIB item for a true
      cross-process denial proof (agent: no — product/scope call). The
      dispatch-level tests inject a synthetic `peer_pid` in-process; there is
      still no multi-process (distinct real-PID) daemon harness under
      `tests/` to prove the `SO_PEERCRED` rejection end-to-end across real
      processes. Noted as a known gap in the CIB-153 block itself
      (`Confidence: medium`).

## Notes

Scope: `session.heartbeat` and `session.unregister` previously had no
peer-credential check, so any same-uid IPC client that guessed a session id
could keep-alive or force-unregister a session it never registered. This
mirrors the existing MLP2-074 `report_process` peer-ownership contract onto
both lifecycle verbs.

Key design point worth preserving on any future touch: ownership binds to a
new immutable `RegistryEntry::launcher_pid` (stamped once at
`register_with_lineage`), not the mutable `record.pid` — the mutable pid is
narrowed by `update_lineage_anchor` when `report_process` re-points the
anchor at a spawned child, which would otherwise strand the launcher's own
heartbeats/unregister after that point. The real `anvil-run` ordering is
register -> report_process -> heartbeats -> unregister, all emitted by the
launcher.

Files touched by the code PR:

- `crates/anvil-intercept/src/ipc.rs` — thread `peer_pid` through
  `dispatch_command`'s `Heartbeat`/`UnregisterSession` arms; new dispatch-level
  tests.
- `crates/anvil-intercept/src/lib.rs` — `RegistryDispatcher` passes `peer_pid`
  through to the registry.
- `crates/anvil-intercept/src/registry.rs` — `SessionDispatcher` trait
  signature change; `RegistryEntry::launcher_pid`; `verify_peer_owns` check;
  registry-level unit tests.
- `crates/anvil-intercept/tests/daemon_config_wired.rs` — warm-state tests
  updated to register with a lineage anchor keyed to the test pid.
- `crates/anvil-intercept/tests/jsonrpc_conformance.rs` — conformance double
  updated for the new `SessionDispatcher` signature.

Local gates run green on this branch:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p eddacraft-anvil-intercept`
